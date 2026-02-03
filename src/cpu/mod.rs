//! 6502 CPU emulation for the NES.
//!
//! Full instruction set including undocumented opcodes; nestest-compatible.
//! Bus trait used for memory and I/O (PPU, APU, cartridge, controller).

pub mod cpu;
pub mod flags;
