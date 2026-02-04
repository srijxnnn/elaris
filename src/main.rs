//! NES emulator entry point.
//!
//! Loads a cartridge and runs the CPU with a display window and audio output.
//! Usage: `elaris [path/to/game.nes]`
//!
//! ## NESdev references
//!
//! - [Cycle reference chart](https://www.nesdev.org/wiki/Cycle_reference_chart): NTSC frame rate
//!   (~60.0988 Hz), CPU/PPU cycle relationship (3 PPU cycles per CPU cycle).
//! - [NMI](https://www.nesdev.org/wiki/NMI): VBlank NMI triggers at scanline 241; games sync to
//!   this for one logic frame per display frame.
//! - [APU](https://www.nesdev.org/wiki/APU): Audio sampled at 44.1 kHz; DMC can stall CPU for
//!   sample fetches (4 cycles per byte from PRG).

use std::env;
use std::path::Path;
use std::time::{Duration, Instant};

use elaris::{bus::Bus, bus::NesBus, cartridge::cartridge::Cartridge, cpu::cpu::CPU};
use minifb::{Key, Window, WindowOptions};
use rodio::OutputStream;

/// NES NTSC frame rate is ~60.0988 Hz. We target 16.67 ms per frame for ~60 fps display.
/// See: NESdev wiki "Cycle reference chart" (frame = 29780.5 CPU cycles at 1.789773 MHz).
const FRAME_DURATION: Duration = Duration::from_nanos(16_666_667);

/// Audio output sample rate (Hz). Matches APU sample generation rate.
/// APU mixer runs at CPU clock; we resample to 44.1 kHz for output (see APU_Mixer).
const SAMPLE_RATE: u32 = 44_100;

/// Build controller port 1 ($4016) button state from keyboard.
/// Bit order matches [Standard controller](https://www.nesdev.org/wiki/Standard_controller):
/// 0=A, 1=B, 2=Select, 3=Start, 4=Up, 5=Down, 6=Left, 7=Right.
/// This state is latched into the shift register when the game writes 1 then 0 to $4016.
fn controller_state_from_keys(window: &Window) -> u8 {
    let mut state = 0u8;
    if window.is_key_down(Key::Z) {
        state |= 1 << 0; // A
    }
    if window.is_key_down(Key::X) {
        state |= 1 << 1; // B
    }
    if window.is_key_down(Key::RightShift) || window.is_key_down(Key::LeftShift) {
        state |= 1 << 2; // Select
    }
    if window.is_key_down(Key::Enter) {
        state |= 1 << 3; // Start
    }
    if window.is_key_down(Key::Up) {
        state |= 1 << 4;
    }
    if window.is_key_down(Key::Down) {
        state |= 1 << 5;
    }
    if window.is_key_down(Key::Left) {
        state |= 1 << 6;
    }
    if window.is_key_down(Key::Right) {
        state |= 1 << 7;
    }
    state
}

fn main() {
    // Load ROM from path or default to nestest for CPU verification (nestest: CPU test ROM).
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "test/nestest.nes".to_string());

    let cart = Cartridge::load(&path);
    let bus = NesBus::new(cart);
    // CPU initial state: A,X,Y=0, SP=$FD, P=$24 (I=1, U=1), PC set by reset vector.
    // See: NESdev "CPU power up state" / "Reset vector" ($FFFC–$FFFD).
    let mut cpu = CPU {
        a: 0,
        x: 0,
        y: 0,
        sp: 0xFD,
        pc: 0,
        status: 0x24,
        cycles: 0,
        bus,
        halted: false,
    };

    // Reset: PC from $FFFC/$FFFD (cartridge supplies vector). Nestest.nes expects entry at $C000.
    if path.contains("nestest") {
        cpu.pc = 0xC000;
        cpu.cycles = 7;
    } else {
        cpu.reset();
    }

    // NES PPU output is 256×240 pixels (8×8 tiles: 32×30 visible). See PPU_registers / PPU_rendering.
    let mut window = Window::new(
        format!(
            "{} - Elaris",
            Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ROM")
        )
        .as_str(),
        256,
        240,
        WindowOptions {
            borderless: true,
            resize: true,
            scale: minifb::Scale::FitScreen,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            topmost: true,
            title: true,
            transparency: false,
            none: false,
        },
    )
    .expect("Failed to create window");

    window.set_target_fps(60);

    // Audio: default device, sink for queueing APU samples each frame
    let (_stream, stream_handle) = OutputStream::try_default().expect("No default audio device");
    let sink = rodio::Sink::try_new(&stream_handle).expect("Failed to create audio sink");
    let mut audio_buf = vec![0.0f32; 1024];

    // Main loop: run one frame of emulation, then present and pace to 60 fps
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        // Keyboard → controller port 1. Game latches by writing 1 then 0 to $4016 (Controller_reading).
        cpu.bus.controller.state = controller_state_from_keys(&window);

        // Run one frame: CPU runs until PPU signals vblank (scanline 241, cycle 1). Each CPU
        // instruction calls bus.tick(cycles), advancing PPU by 3× cycles and APU by cycles.
        while !cpu.bus.frame_ready() {
            // DMC sample fetch: when buffer empty, APU requests a byte. CPU is stalled 4 cycles
            // while the DMC reads from PRG ($8000–$FFFF). See APU_DMC "Memory reader".
            while let Some(addr) = cpu.bus.apu.dmc_wants_fetch() {
                cpu.cycles += 4;
                for _ in 0..4 {
                    cpu.bus.tick(1);
                }
                let byte = cpu.bus.read(addr);
                cpu.bus.apu.dmc_feed_byte(byte);
            }
            cpu.step();
            if cpu.halted {
                break;
            }
        }
        if cpu.halted {
            break;
        }

        if cpu.bus.frame_ready() {
            // Framebuffer was filled as each visible scanline (0–239) completed; vblank flag set
            // at scanline 241. We present the buffer and clear frame_ready for next frame.
            window
                .update_with_buffer(&cpu.bus.ppu.framebuffer, 256, 240)
                .expect("Failed to update window");
            cpu.bus.clear_frame_ready();

            // APU samples are 0..1 (mixer output); convert to -1..1 for rodio playback.
            let n = cpu.bus.apu.drain_samples(&mut audio_buf);
            if n > 0 {
                let samples: Vec<f32> = audio_buf[..n]
                    .iter()
                    .map(|s| (s * 2.0 - 1.0).clamp(-1.0, 1.0))
                    .collect();
                let source = rodio::buffer::SamplesBuffer::new(1, SAMPLE_RATE, samples);
                sink.append(source);
            }
        }

        // Pace to ~60 fps so we don't burn CPU (emulation is far faster than real NES)
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }
}
