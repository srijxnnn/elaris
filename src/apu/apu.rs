//! NES APU: pulse (x2), triangle, noise, frame counter, and mixer.
//!
//! APU runs at CPU clock; pulse/noise timers and frame counter run at half rate (1 APU cycle = 2 CPU cycles).
//! Triangle timer runs at CPU rate. See NESdev wiki for details.

/// NTSC CPU clock ~1.789773 MHz. Sample rate 44100 → ~40.56 cycles per sample.
const CYCLES_PER_SAMPLE: f64 = 1_789_773.0 / 44_100.0;

/// Length counter lookup (indices 0x00..0x1F). From NESdev.
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

/// Noise period table (NTSC), CPU cycles. From NESdev.
const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// Pulse duty sequences (8 steps each). Sequencer reads in order 0,7,6,5,4,3,2,1.
const PULSE_DUTY: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [0, 0, 0, 0, 0, 0, 1, 1], // 25%
    [0, 0, 0, 0, 1, 1, 1, 1], // 50%
    [1, 1, 1, 1, 1, 1, 0, 0], // 25% negated
];

/// Triangle wave 32-step sequence (0-15 then 15-0).
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

/// 4-step frame counter: cycle counts at which quarter/half frame and IRQ fire (CPU cycles).
const FRAME_4STEP_RESET: u32 = 29830;

/// 5-step frame counter: no IRQ; reset after last step (CPU cycles).
const FRAME_5STEP_RESET: u32 = 37282;

// -----------------------------------------------------------------------------
// Pulse channel ($4000–$4003 / $4004–$4007): duty, envelope, sweep, length, timer
// -----------------------------------------------------------------------------

/// Pulse channel state. Timer runs at APU cycle rate (every 2 CPU cycles).
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
        self.length_counter = LENGTH_TABLE[(data >> 3) as usize & 0x1F];
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
        if self.sweep_reload {
            if self.sweep_divider == 0 {
                self.sweep_reload = false;
            }
            self.sweep_divider = self.sweep_period;
        } else if self.sweep_divider > 0 {
            self.sweep_divider -= 1;
        } else {
            self.sweep_divider = self.sweep_period;
            if self.sweep_enable && self.sweep_shift > 0 {
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
            self.sweep_reload = true;
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
        self.sequencer_step = (self.sequencer_step + 1) & 7;
        false
    }
}

// -----------------------------------------------------------------------------
// Triangle channel ($4008–$400B): linear counter, length, 32-step waveform
// -----------------------------------------------------------------------------

/// Triangle channel state. Timer runs at CPU cycle rate.
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
        self.length_counter = LENGTH_TABLE[(data >> 3) as usize & 0x1F];
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
        if self.length_halt {
            self.linear_reload = false;
        }
    }

    fn output(&self) -> u8 {
        if !self.enabled
            || self.length_counter == 0
            || self.linear_counter == 0
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
// Noise channel ($400C–$400F): envelope, LFSR, period table, length counter
// -----------------------------------------------------------------------------

/// Noise channel state. LFSR is 15-bit; timer uses NOISE_PERIOD_TABLE (CPU cycles).
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
        self.length_counter = LENGTH_TABLE[(data >> 3) as usize & 0x1F];
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
        let period = NOISE_PERIOD_TABLE[self.period_index as usize];
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
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
// Mixer: NES non-linear formulas (pulse and TND groups)
// -----------------------------------------------------------------------------

/// Pulse mixer lookup: 95.52 / (8128/n + 100) for n = pulse1 + pulse2 (0..31). n=0 → 0.
fn pulse_table(n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    95.52 / (8128.0 / (n as f32) + 100.0)
}

/// TND mixer lookup: 163.67 / (24329/n + 100). n = 3×triangle + 2×noise + dmc (dmc=0 here).
fn tnd_table(n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    163.67 / (24329.0 / (n as f32) + 100.0)
}

// -----------------------------------------------------------------------------
// APU: register dispatch, frame counter, tick, sample buffer
// -----------------------------------------------------------------------------

/// APU state: all channels, frame counter, status, and sample buffer for 44.1 kHz output.
pub struct APU {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
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
            noise: Noise { shift: 1, ..Noise::default() },
            status: 0,
            frame_irq_inhibit: false,
            frame_4step: true,
            frame_cycle: 0,
            sample_phase: 0.0,
            sample_buffer: Vec::new(),
        }
    }

    /// Write to APU registers $4000–$4017. Called from bus on CPU write.
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

    /// Read $4015: length counter status (bits 0–3), frame IRQ (bit 6). Clears frame IRQ.
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
        self.status &= 0x3F;
        r
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
        let tnd = 3 * (tri as usize) + 2 * (noi as usize) + 0;
        let pulse_out = pulse_table(pulse_sum.min(31));
        let tnd_out = tnd_table(tnd.min(203));
        let out = pulse_out + tnd_out;
        // Scale to 0..1 and apply gain so NES-level output is clearly audible
        ((out / 255.0) * 1.8).min(1.0)
    }

    /// Advance APU by `cycles` CPU cycles: frame counter, channel timers, mixer samples.
    /// Pushes one sample every ~40.56 CPU cycles (44.1 kHz).
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
