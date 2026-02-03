//! NES APU (Audio Processing Unit) implementation.
//!
//! Implements the [APU](https://www.nesdev.org/wiki/APU) as in the Ricoh 2A03: five channels (pulse×2,
//! triangle, noise, DMC), [frame counter](https://www.nesdev.org/wiki/APU_Frame_Counter) (4-step or
//! 5-step), and [APU Mixer](https://www.nesdev.org/wiki/APU_Mixer) (non-linear). Registers $4000–$4013,
//! $4015, $4017. See [APU registers](https://www.nesdev.org/wiki/APU_registers).
//!
//! ## Timing
//!
//! - Pulse and noise: timer clocked every 2 CPU cycles (APU "half cycle").
//! - Triangle: timer at CPU rate. Length/envelope/sweep clocked by frame counter (~240 Hz).
//! - DMC: rate from lookup table; when sample buffer empty, CPU is stalled 4 cycles for PRG read.
//!
//! ## References
//!
//! - [APU Pulse](https://www.nesdev.org/wiki/APU_Pulse), [APU Triangle](https://www.nesdev.org/wiki/APU_Triangle)
//! - [APU Noise](https://www.nesdev.org/wiki/APU_Noise), [APU DMC](https://www.nesdev.org/wiki/APU_DMC)
//! - [APU Length Counter](https://www.nesdev.org/wiki/APU_Length_Counter), [APU Envelope](https://www.nesdev.org/wiki/APU_Envelope)

/// NTSC CPU clock ~1.789773 MHz. We generate one sample every CYCLES_PER_SAMPLE CPU cycles (~40.56)
/// to get 44.1 kHz output. See Cycle_reference_chart.
const CYCLES_PER_SAMPLE: f64 = 1_789_773.0 / 44_100.0;

/// Length counter lookup table: 5-bit index from register → count. APU_Length_Counter.
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

/// Noise channel period table (NTSC): 4-bit index from $400E → period in CPU cycles. APU_Noise.
const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// Pulse channel duty cycles (8 steps). Duty 0=12.5%, 1=25%, 2=50%, 3=25% negated. Sequencer steps
/// 0→7→6→…→1. APU_Pulse. Output is volume when step is 1, else 0.
const PULSE_DUTY: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [0, 0, 0, 0, 0, 0, 1, 1], // 25%
    [0, 0, 0, 0, 1, 1, 1, 1], // 50%
    [1, 1, 1, 1, 1, 1, 0, 0], // 25% negated
];

/// Triangle channel 32-step waveform: 15 down to 0, then 0 up to 15. No volume control. APU_Triangle.
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

/// 4-step frame counter: resets every 29830 CPU cycles. Quarter/half frame at 7457, 14913, 22371;
/// IRQ (if not inhibited) at 29829. APU_Frame_Counter.
const FRAME_4STEP_RESET: u32 = 29830;

/// 5-step frame counter: no IRQ; resets every 37282 cycles. Extra half-frame at 37281.
const FRAME_5STEP_RESET: u32 = 37282;

// -----------------------------------------------------------------------------
// Pulse channel ($4000–$4003 = pulse 1, $4004–$4007 = pulse 2)
// Duty, envelope, sweep, length counter, 11-bit timer. Timer clocked every 2 CPU cycles.
// -----------------------------------------------------------------------------

/// Pulse channel: square wave with configurable duty, volume/envelope, frequency sweep, length counter.
#[derive(Default)]
struct Pulse {
    enabled: bool,
    duty: u8,
    length_halt: bool,
    constant_volume: bool,
    volume: u8,
    sweep_enable: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    timer_period: u16,
    timer: u16,
    sequencer_step: u8,
    length_counter: u8,
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
    sweep_divider: u8,
    sweep_reload: bool,
}

impl Pulse {
    /// $4000/$4004: duty, length halt, constant volume, volume/envelope period.
    fn write_4000(&mut self, data: u8) {
        self.duty = (data >> 6) & 3;
        self.length_halt = data & 0x20 != 0;
        self.constant_volume = data & 0x10 != 0;
        self.volume = data & 0x0F;
        self.envelope_start = true;
    }

    /// $4001/$4005: sweep enable, period, negate, shift.
    fn write_4001(&mut self, data: u8) {
        self.sweep_enable = data & 0x80 != 0;
        self.sweep_period = (data >> 4) & 7;
        self.sweep_negate = data & 0x08 != 0;
        self.sweep_shift = data & 7;
        self.sweep_reload = true;
    }

    /// $4002/$4006: timer low 8 bits.
    fn write_4002(&mut self, data: u8) {
        self.timer_period = (self.timer_period & 0x0700) | data as u16;
    }

    /// $4003/$4007: length counter load, timer high 3 bits; restarts envelope and sequencer.
    fn write_4003(&mut self, data: u8) {
        self.timer_period = (self.timer_period & 0x00FF) | ((data & 7) as u16) << 8;
        if self.enabled {
            self.length_counter = LENGTH_TABLE[(data >> 3) as usize & 0x1F];
        }
        self.envelope_start = true;
        self.sequencer_step = 0;
    }

    fn clock_length(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
            self.envelope_start = false;
        } else if self.envelope_divider > 0 {
            self.envelope_divider -= 1;
        } else {
            self.envelope_divider = self.volume;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.length_halt {
                self.envelope_decay = 15;
            }
        }
    }

    fn clock_sweep(&mut self) -> bool {
        let mut silence = false;

        // Save whether divider is currently zero (before reload/decrement)
        let divider_was_zero = self.sweep_divider == 0;

        // Step 1: Reload divider or decrement
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }

        // Step 2: When divider was zero, adjust period if sweep enabled
        if divider_was_zero && self.sweep_enable && self.sweep_shift > 0 {
            let delta = self.timer_period >> self.sweep_shift;
            if self.sweep_negate {
                self.timer_period = self.timer_period.saturating_sub(delta);
            } else {
                self.timer_period = self.timer_period.saturating_add(delta);
            }
            if self.timer_period > 0x7FF {
                silence = true;
            }
        }

        silence
    }

    fn output(&self, sweep_silence: bool) -> u8 {
        if !self.enabled
            || self.length_counter == 0
            || sweep_silence
            || self.timer_period < 8
            || PULSE_DUTY[self.duty as usize][self.sequencer_step as usize] == 0
        {
            return 0;
        }
        if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        }
    }

    fn tick_apu_cycle(&mut self) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
            return false;
        }
        self.timer = self.timer_period;
        self.sequencer_step = (self.sequencer_step.wrapping_sub(1)) & 7;
        false
    }
}

// -----------------------------------------------------------------------------
// Triangle channel ($4008–$400B): linear counter (7-bit), length counter, 32-step triangle wave
// Timer runs at CPU cycle rate (one octave below pulse for same period).
// -----------------------------------------------------------------------------

/// Triangle channel: 32-step triangle wave, linear counter + length counter, no volume control.
#[derive(Default)]
struct Triangle {
    enabled: bool,
    length_halt: bool,
    linear_load: u8,
    timer_period: u16,
    timer: u16,
    length_counter: u8,
    linear_counter: u8,
    linear_reload: bool,
    sequencer_step: u8,
}

impl Triangle {
    /// $4008: length halt, linear counter load value.
    fn write_4008(&mut self, data: u8) {
        self.length_halt = data & 0x80 != 0;
        self.linear_load = data & 0x7F;
    }

    /// $400A: timer low 8 bits.
    fn write_400a(&mut self, data: u8) {
        self.timer_period = (self.timer_period & 0xFF00) | data as u16;
    }

    /// $400B: length counter load, timer high 3 bits; sets linear reload flag.
    fn write_400b(&mut self, data: u8) {
        self.timer_period = (self.timer_period & 0x00FF) | ((data & 7) as u16) << 8;
        if self.enabled {
            self.length_counter = LENGTH_TABLE[(data >> 3) as usize & 0x1F];
        }
        self.linear_reload = true;
    }

    fn clock_length(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn clock_linear(&mut self) {
        if self.linear_reload {
            self.linear_counter = self.linear_load;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        if !self.length_halt {
            self.linear_reload = false;
        }
    }

    fn output(&self) -> u8 {
        if !self.enabled
            || self.length_counter == 0
            || self.linear_counter == 0
            || self.timer_period < 2
        {
            return 0;
        }
        TRIANGLE_SEQUENCE[self.sequencer_step as usize]
    }

    fn tick_cpu_cycle(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        self.timer = self.timer_period;
        if self.length_counter > 0 && self.linear_counter > 0 {
            self.sequencer_step = (self.sequencer_step + 1) & 31;
        }
    }
}

// -----------------------------------------------------------------------------
// Noise channel ($400C–$400F): envelope, 15-bit LFSR, period from $400E, length counter
// -----------------------------------------------------------------------------

/// Noise channel: pseudo-random output from 15-bit LFSR; mode bit shortens period (metallic tone).
#[derive(Default)]
struct Noise {
    enabled: bool,
    length_halt: bool,
    constant_volume: bool,
    volume: u8,
    mode: bool,
    period_index: u8,
    length_counter: u8,
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
    timer: u16,
    shift: u16,
}

impl Noise {
    /// $400C: length halt, constant volume, volume/envelope.
    fn write_400c(&mut self, data: u8) {
        self.length_halt = data & 0x20 != 0;
        self.constant_volume = data & 0x10 != 0;
        self.volume = data & 0x0F;
        self.envelope_start = true;
    }

    /// $400E: LFSR mode (bit 7), period index (bits 0–3).
    fn write_400e(&mut self, data: u8) {
        self.mode = data & 0x80 != 0;
        self.period_index = data & 0x0F;
    }

    /// $400F: length counter load; restarts envelope.
    fn write_400f(&mut self, data: u8) {
        if self.enabled {
            self.length_counter = LENGTH_TABLE[(data >> 3) as usize & 0x1F];
        }
        self.envelope_start = true;
    }

    fn clock_length(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
            self.envelope_start = false;
        } else if self.envelope_divider > 0 {
            self.envelope_divider -= 1;
        } else {
            self.envelope_divider = self.volume;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.length_halt {
                self.envelope_decay = 15;
            }
        }
    }

    fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || (self.shift & 1) != 0 {
            return 0;
        }
        if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        }
    }

    fn tick_cpu_cycle(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        let period = NOISE_PERIOD_TABLE[self.period_index as usize];
        self.timer = period;
        let feedback = if self.mode {
            (self.shift & 1) ^ ((self.shift >> 6) & 1)
        } else {
            (self.shift & 1) ^ ((self.shift >> 1) & 1)
        };
        self.shift = (self.shift >> 1) | (feedback << 14);
    }
}

// -----------------------------------------------------------------------------
// DMC channel ($4010–$4013): delta modulation, 7-bit output, sample buffer, CPU stall on fetch
// Sample address $C000 + (byte*64); length (byte*16)+1. APU_DMC.
// -----------------------------------------------------------------------------

/// DMC rate table (NTSC): 4-bit index from $4010 → CPU cycles per output bit. APU_DMC.
const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

struct Dmc {
    irq_enable: bool,
    loop_flag: bool,
    rate_index: u8,
    rate_timer: u16,
    output_level: u8,
    start_address: u16,
    sample_length: u16,
    current_address: u16,
    bytes_remaining: u16,
    /// Single-byte sample buffer; filled by memory reader, emptied into shift_register when output cycle starts.
    sample_buffer: Option<u8>,
    shift_register: u8,
    bits_remaining: u8,
    /// True when buffer is empty (no sample byte to output). Power-on and when waiting for fetch: output 0.
    silence: bool,
    enabled: bool,
    /// When true, the bus must stall CPU 4 cycles, read from fetch_address, and call dmc_feed_byte.
    fetch_pending: bool,
    fetch_address: u16,
}

impl Default for Dmc {
    fn default() -> Self {
        Self {
            irq_enable: false,
            loop_flag: false,
            rate_index: 0,
            rate_timer: 0,
            output_level: 0,
            start_address: 0,
            sample_length: 0,
            current_address: 0,
            bytes_remaining: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 0,
            silence: true, // power-on: output 0 until we have sample data
            enabled: false,
            fetch_pending: false,
            fetch_address: 0,
        }
    }
}

impl Dmc {
    /// $4010: IRQ enable (bit 7), loop (bit 6), rate index (bits 0–3). Clearing IRQ enable clears DMC IRQ.
    fn write_4010(&mut self, data: u8, status: &mut u8) {
        self.irq_enable = data & 0x80 != 0;
        if !self.irq_enable {
            *status &= 0x7F;
        }
        self.loop_flag = data & 0x40 != 0;
        self.rate_index = data & 0x0F;
    }

    /// $4011: Direct load — set output level to lower 7 bits.
    fn write_4011(&mut self, data: u8) {
        self.output_level = data & 0x7F;
    }

    /// $4012: Sample address = $C000 + (value * 64).
    fn write_4012(&mut self, data: u8) {
        self.start_address = 0xC000 + (data as u16) * 64;
    }

    /// $4013: Sample length = (value * 16) + 1 bytes.
    fn write_4013(&mut self, data: u8) {
        self.sample_length = (data as u16) * 16 + 1;
    }

    /// Enable/disable from $4015 bit 4. When enabled and bytes_remaining == 0, (re)start sample.
    fn set_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.enabled = false;
            self.fetch_pending = false;
            return;
        }
        self.enabled = true;
        if self.bytes_remaining == 0 {
            self.current_address = self.start_address;
            self.bytes_remaining = self.sample_length;
            self.fetch_pending = self.sample_buffer.is_none() && self.bytes_remaining > 0;
            if self.fetch_pending {
                self.fetch_address = self.current_address;
            }
        }
    }

    /// Called by bus after stalling 4 cycles and reading the byte from PRG. Address wrap $FFFF -> $8000.
    fn feed_byte(&mut self, byte: u8, status: &mut u8) {
        self.fetch_pending = false;
        self.sample_buffer = Some(byte);
        self.current_address = self.current_address.wrapping_add(1);
        if self.current_address == 0 {
            self.current_address = 0x8000;
        }
        if self.bytes_remaining > 0 {
            self.bytes_remaining -= 1;
        }
        if self.bytes_remaining == 0 {
            if self.loop_flag {
                self.current_address = self.start_address;
                self.bytes_remaining = self.sample_length;
            } else if self.irq_enable {
                *status |= 0x80;
            }
        }
    }

    /// Run one CPU cycle: count down rate timer; when it hits 0, output one bit (or silence) and possibly start new cycle / request fetch.
    fn tick(&mut self) {
        if !self.enabled {
            return;
        }
        if self.rate_timer > 0 {
            self.rate_timer -= 1;
            return;
        }
        let period = DMC_RATE_TABLE[self.rate_index as usize];
        self.rate_timer = period.saturating_sub(1);

        if !self.silence {
            let bit = self.shift_register & 1;
            if bit != 0 {
                if self.output_level <= 125 {
                    self.output_level += 2;
                }
            } else if self.output_level >= 2 {
                self.output_level -= 2;
            }
        }
        self.shift_register >>= 1;

        if self.bits_remaining > 0 {
            self.bits_remaining -= 1;
        }
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            if let Some(byte) = self.sample_buffer.take() {
                self.shift_register = byte;
                self.silence = false;
            } else {
                self.silence = true;
            }
            if self.sample_buffer.is_none() && self.bytes_remaining > 0 {
                self.fetch_pending = true;
                self.fetch_address = self.current_address;
            }
        }
    }

    /// Output for mixer: 0 when silence, else 7-bit level (0–127). Sent to mixer whether enabled or not.
    fn output(&self) -> u8 {
        if self.silence { 0 } else { self.output_level }
    }

    fn has_bytes_remaining(&self) -> bool {
        self.bytes_remaining > 0 || self.sample_buffer.is_some()
    }
}

// -----------------------------------------------------------------------------
// Mixer: NES non-linear combination (APU_Mixer). Pulse group and TND group combined.
// -----------------------------------------------------------------------------

/// Pulse output: 95.52 / (8128/n + 100), n = pulse1 + pulse2 (0–31). APU_Mixer.
fn pulse_table(n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    95.52 / (8128.0 / (n as f32) + 100.0)
}

/// TND (triangle + noise + DMC) output: 163.67 / (24329/n + 100), n = 3*tri + 2*noise + dmc.
fn tnd_table(n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    163.67 / (24329.0 / (n as f32) + 100.0)
}

// -----------------------------------------------------------------------------
// APU: register dispatch, frame counter, tick, sample buffer
// -----------------------------------------------------------------------------

/// APU state: pulse×2, triangle, noise, DMC; frame counter; status ($4015); sample buffer for
/// 44.1 kHz output. tick(cycles) advances frame counter and channels, pushes samples when due.
pub struct APU {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,
    status: u8,
    frame_irq_inhibit: bool,
    frame_4step: bool,
    frame_cycle: u32,
    sample_phase: f64,
    pub sample_buffer: Vec<f32>,
}

impl Default for APU {
    fn default() -> Self {
        Self::new()
    }
}

impl APU {
    pub fn new() -> Self {
        Self {
            pulse1: Pulse::default(),
            pulse2: Pulse::default(),
            triangle: Triangle::default(),
            noise: Noise {
                shift: 1,
                ..Noise::default()
            },
            dmc: Dmc::default(),
            status: 0,
            frame_irq_inhibit: false,
            frame_4step: true,
            frame_cycle: 0,
            sample_phase: 0.0,
            sample_buffer: Vec::new(),
        }
    }

    /// Write to APU registers. $4000–$4013 = channel regs; $4015 = enable + length status;
    /// $4017 = frame counter (mode 4/5-step, IRQ inhibit). Writing $4017 resets frame counter.
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000 => self.pulse1.write_4000(data),
            0x4001 => self.pulse1.write_4001(data),
            0x4002 => self.pulse1.write_4002(data),
            0x4003 => self.pulse1.write_4003(data),
            0x4004 => self.pulse2.write_4000(data),
            0x4005 => self.pulse2.write_4001(data),
            0x4006 => self.pulse2.write_4002(data),
            0x4007 => self.pulse2.write_4003(data),
            0x4008 => self.triangle.write_4008(data),
            0x400A => self.triangle.write_400a(data),
            0x400B => self.triangle.write_400b(data),
            0x400C => self.noise.write_400c(data),
            0x400E => self.noise.write_400e(data),
            0x400F => self.noise.write_400f(data),
            0x4010 => self.dmc.write_4010(data, &mut self.status),
            0x4011 => self.dmc.write_4011(data),
            0x4012 => self.dmc.write_4012(data),
            0x4013 => self.dmc.write_4013(data),
            0x4015 => {
                self.pulse1.enabled = data & 1 != 0;
                self.pulse2.enabled = data & 2 != 0;
                self.triangle.enabled = data & 4 != 0;
                self.noise.enabled = data & 8 != 0;
                if !self.pulse1.enabled {
                    self.pulse1.length_counter = 0;
                }
                if !self.pulse2.enabled {
                    self.pulse2.length_counter = 0;
                }
                if !self.triangle.enabled {
                    self.triangle.length_counter = 0;
                }
                if !self.noise.enabled {
                    self.noise.length_counter = 0;
                }
                self.dmc.set_enabled(data & 0x10 != 0);
            }
            0x4017 => {
                self.frame_4step = data & 0x80 == 0;
                self.frame_irq_inhibit = data & 0x40 != 0;
                self.frame_cycle = 0;
                if self.frame_irq_inhibit {
                    self.status &= !0x40;
                }
                // When 5-step mode is selected (bit 7 set), one quarter and one half frame are
                // generated immediately (after 3–4 cycles on real hardware; we do it at once).
                if !self.frame_4step {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
            }
            _ => {}
        }
    }

    /// Read $4015: bits 0–3 = length counter > 0 for pulse1, pulse2, triangle, noise; bit 4 = DMC
    /// has bytes remaining; bit 6 = frame IRQ; bit 7 = DMC IRQ. Reading clears frame and DMC IRQ.
    pub fn read_status(&mut self) -> u8 {
        let mut r = self.status & 0xC0;
        if self.pulse1.length_counter > 0 {
            r |= 0x01;
        }
        if self.pulse2.length_counter > 0 {
            r |= 0x02;
        }
        if self.triangle.length_counter > 0 {
            r |= 0x04;
        }
        if self.noise.length_counter > 0 {
            r |= 0x08;
        }
        if self.dmc.has_bytes_remaining() {
            r |= 0x10;
        }
        self.status &= 0x3F;
        r
    }

    /// DMC memory reader: when sample buffer is empty and bytes_remaining > 0, returns Some(addr)
    /// for the bus to read from PRG. Bus must stall CPU 4 cycles, read byte, call dmc_feed_byte.
    /// See APU_DMC "Memory reader".
    pub fn dmc_wants_fetch(&self) -> Option<u16> {
        if self.dmc.fetch_pending {
            Some(self.dmc.fetch_address)
        } else {
            None
        }
    }

    /// Feed a byte read from PRG into the DMC after a requested fetch. Call only when dmc_wants_fetch() returned Some.
    pub fn dmc_feed_byte(&mut self, byte: u8) {
        self.dmc.feed_byte(byte, &mut self.status);
    }

    /// Quarter-frame: clock envelope (pulse, noise) and triangle linear counter.
    fn clock_quarter_frame(&mut self) {
        self.pulse1.clock_envelope();
        self.pulse2.clock_envelope();
        self.noise.clock_envelope();
        self.triangle.clock_linear();
    }

    /// Half-frame: clock length counters and sweep units.
    fn clock_half_frame(&mut self) {
        self.pulse1.clock_length();
        self.pulse2.clock_length();
        self.triangle.clock_length();
        self.noise.clock_length();
        self.pulse1.clock_sweep();
        self.pulse2.clock_sweep();
    }

    fn mix(&self) -> f32 {
        let sweep_silence1 = self.pulse1.timer_period > 0x7FF;
        let sweep_silence2 = self.pulse2.timer_period > 0x7FF;
        let p1 = self.pulse1.output(sweep_silence1);
        let p2 = self.pulse2.output(sweep_silence2);
        let pulse_sum = (p1 + p2) as usize;
        let tri = self.triangle.output();
        let noi = self.noise.output();
        let dmc = self.dmc.output() as usize;
        let tnd = 3 * (tri as usize) + 2 * (noi as usize) + dmc;
        let pulse_out = pulse_table(pulse_sum.min(31));
        let tnd_out = tnd_table(tnd.min(203));
        let out = pulse_out + tnd_out;
        // Scale to 0..1 and apply moderate gain
        (out / 255.0).min(1.0)
    }

    /// Advance APU by `cycles` CPU cycles: frame counter (quarter/half frame, IRQ), pulse/noise/triangle/DMC
    /// timers, and mixer. One sample pushed every CYCLES_PER_SAMPLE cycles (~44.1 kHz).
    pub fn tick(&mut self, cycles: usize) {
        let cycles = cycles as u32;
        for _ in 0..cycles {
            self.frame_cycle += 1;
            let apu_half_cycle = self.frame_cycle % 2 == 0;

            if self.frame_4step {
                match self.frame_cycle {
                    7457 => self.clock_quarter_frame(),
                    14913 => {
                        self.clock_quarter_frame();
                        self.clock_half_frame();
                    }
                    22371 => self.clock_quarter_frame(),
                    29829 => {
                        self.clock_half_frame();
                        if !self.frame_irq_inhibit {
                            self.status |= 0x40;
                        }
                    }
                    _ => {}
                }
                if self.frame_cycle >= FRAME_4STEP_RESET {
                    self.frame_cycle = 0;
                }
            } else {
                // 5-step mode (used by Donkey Kong and many other games)
                match self.frame_cycle {
                    7457 => self.clock_quarter_frame(),
                    14913 => {
                        self.clock_quarter_frame();
                        self.clock_half_frame();
                    }
                    22371 => self.clock_quarter_frame(),
                    29829 => {}
                    37281 => {
                        self.clock_quarter_frame();
                        self.clock_half_frame();
                    }
                    _ => {}
                }
                if self.frame_cycle >= FRAME_5STEP_RESET {
                    self.frame_cycle = 0;
                }
            }

            if apu_half_cycle {
                self.pulse1.tick_apu_cycle();
                self.pulse2.tick_apu_cycle();
            }
            self.triangle.tick_cpu_cycle();
            self.noise.tick_cpu_cycle();
            self.dmc.tick();

            self.sample_phase += 1.0;
            if self.sample_phase >= CYCLES_PER_SAMPLE {
                self.sample_phase -= CYCLES_PER_SAMPLE;
                self.sample_buffer.push(self.mix());
            }
        }
    }

    /// Drain samples from the internal buffer into `out`. Returns number of samples copied.
    pub fn drain_samples(&mut self, out: &mut [f32]) -> usize {
        let n = out.len().min(self.sample_buffer.len());
        out[..n].copy_from_slice(&self.sample_buffer[..n]);
        self.sample_buffer.drain(..n);
        n
    }
}
