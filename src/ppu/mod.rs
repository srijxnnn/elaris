//! PPU (Picture Processing Unit) emulation for the NES.
//!
//! See [PPU](https://www.nesdev.org/wiki/PPU), [PPU registers](https://www.nesdev.org/wiki/PPU_registers),
//! [PPU memory map](https://www.nesdev.org/wiki/PPU_memory_map). Handles 341-dot scanlines, 262
//! scanlines per frame, vblank NMI, background and sprite rendering, OAM, nametables, and palette.

pub mod ppu;
