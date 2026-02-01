//! NES emulator entry point.
//!
//! Loads a cartridge and runs the CPU with a display window.
//! Usage: elaris [path/to/game.nes]

use std::env;
use std::time::{Duration, Instant};

use elaris::{bus::NesBus, cartridge::cartridge::Cartridge, cpu::cpu::CPU};
use minifb::{Key, Window, WindowOptions};

/// NES runs at ~60.0988 Hz (NTSC). Target one frame per 16.67 ms for ~60 fps.
const FRAME_DURATION: Duration = Duration::from_nanos(16_666_667);

fn main() {
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

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        // Run until PPU enters vblank (scanline 241)
        while !cpu.bus.frame_ready() {
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
        }

        // Pace to ~60 fps so we don't burn CPU (emulation is far faster than real NES)
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }
}
