//! NES emulator entry point.
//!
//! Loads a cartridge and runs the CPU with a display window and audio output.
//! Usage: `elaris [path/to/game.nes]`

use std::env;
use std::time::{Duration, Instant};

use elaris::{bus::Bus, bus::NesBus, cartridge::cartridge::Cartridge, cpu::cpu::CPU};
use minifb::{Key, Window, WindowOptions};
use rodio::OutputStream;

/// NES frame rate ~60.0988 Hz (NTSC). Target one frame per 16.67 ms for ~60 fps display.
const FRAME_DURATION: Duration = Duration::from_nanos(16_666_667);

/// Audio output sample rate (Hz). Matches APU sample generation rate.
const SAMPLE_RATE: u32 = 44_100;

/// NES controller 1 bits: 0=A, 1=B, 2=Select, 3=Start, 4=Up, 5=Down, 6=Left, 7=Right.
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
    // Load ROM from path or default to nestest for CPU verification
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "test/nestest.nes".to_string());

    let cart = Cartridge::load(&path);
    let bus = NesBus::new(cart);
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

    // Start from reset vector (or nestest entry for nestest.nes)
    if path.contains("nestest") {
        cpu.pc = 0xC000;
        cpu.cycles = 7;
    } else {
        cpu.reset();
    }

    // NES native resolution 256×240
    let mut window = Window::new(
        "Elaris",
        256,
        240,
        WindowOptions {
            borderless: true,
            resize: true,
            scale: minifb::Scale::FitScreen,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            topmost: true,
            title: false,
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

        // Keyboard → controller port 1 (latched when game writes to $4016)
        cpu.bus.controller.state = controller_state_from_keys(&window);

        // Run CPU until PPU enters vblank (scanline 241); PPU/APU tick on each bus.tick()
        while !cpu.bus.frame_ready() {
            // DMC memory reader: when buffer empty, stall CPU 4 cycles and read one byte from PRG
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
            // Framebuffer was filled scanline-by-scanline during the frame (real NES behavior)
            window
                .update_with_buffer(&cpu.bus.ppu.framebuffer, 256, 240)
                .expect("Failed to update window");
            cpu.bus.clear_frame_ready();

            // Push APU samples to audio output (convert 0..1 to -1..1 for proper playback)
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
