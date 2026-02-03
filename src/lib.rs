//! Elaris: An NES (Nintendo Entertainment System) emulator written in Rust.
//!
//! Implements the NES chipset as documented on the
//! [NESdev Wiki](https://www.nesdev.org/wiki/NES_reference_guide): Ricoh 2A03 (CPU+APU),
//! 2C02 PPU, cartridge mappers, and controller I/O.
//!
//! ## Modules (NESdev references)
//!
//! - **apu** – [APU](https://www.nesdev.org/wiki/APU): pulse×2, triangle, noise, DMC, frame
//!   counter, [APU Mixer](https://www.nesdev.org/wiki/APU_Mixer)
//! - **bus** – [CPU memory map](https://www.nesdev.org/wiki/CPU_memory_map): RAM, PPU, APU,
//!   controller, cartridge; 3 PPU cycles per CPU cycle
//! - **cartridge** – [iNES](https://www.nesdev.org/wiki/INES) loading; [Mapper](https://www.nesdev.org/wiki/Mapper) NROM (0), MMC1 (1)
//! - **controller** – [Controller reading](https://www.nesdev.org/wiki/Controller_reading): $4016 latch, shift-out
//! - **cpu** – [6502](https://www.nesdev.org/wiki/CPU) / 2A03: full + undocumented opcodes, [NMI](https://www.nesdev.org/wiki/NMI)
//! - **ppu** – [PPU](https://www.nesdev.org/wiki/PPU), [PPU registers](https://www.nesdev.org/wiki/PPU_registers), OAM, nametables, 256×240

pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod ppu;