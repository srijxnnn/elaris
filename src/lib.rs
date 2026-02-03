//! Elaris: An NES (Nintendo Entertainment System) emulator written in Rust.
//!
//! ## Modules
//!
//! - **apu** – Audio (pulse, triangle, noise, frame counter, mixer)
//! - **bus** – Memory map, PPU/APU/controller/cartridge access
//! - **cartridge** – iNES loading, mappers (NROM, MMC1)
//! - **controller** – NES controller shift-register protocol
//! - **cpu** – 6502 instruction set and execution
//! - **ppu** – Background, sprites, palettes, framebuffer

pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod ppu;