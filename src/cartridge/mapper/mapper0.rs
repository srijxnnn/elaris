//! Mapper 0 (NROM): no bank switching.
//!
//! [NROM](https://www.nesdev.org/wiki/NROM): 16 or 32 KiB PRG ROM at $8000–$FFFF (16 KiB mirrored
//! to fill 32 KiB); 8 KiB CHR at $0000–$1FFF. Some boards have CHR RAM instead of CHR ROM (writable).
//! Mirroring is fixed by board (we default to horizontal). Simplest mapper; used by many early games.

use crate::cartridge::mapper::{Mirroring, mapper::Mapper};

/// NROM: one or two 16 KiB PRG banks, 8 KiB CHR (ROM or RAM). No registers.
/// Mirroring is fixed by the board; we take it from the iNES header (byte 6 bit 0).
pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: Mirroring,
}

impl Mapper0 {
    /// Create NROM with given PRG, CHR, and mirroring. CHR may be ROM or used as 8 KiB RAM (writable) if
    /// cartridge has no CHR ROM (chr_rom.len() == 8192 and we allow writes).
    /// Mirroring should come from iNES header byte 6 bit 0 (0 = horizontal, 1 = vertical).
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            prg_rom,
            chr_rom,
            mirroring,
        }
    }
}

impl Mapper for Mapper0 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            // PRG: $8000–$FFFF. If only 16 KiB, $8000–$BFFF and $C000–$FFFF mirror same bank.
            0x8000..=0xFFFF => {
                let mut addr = (addr - 0x8000) as usize;
                if self.prg_rom.len() == 16 * 1024 {
                    addr %= 16 * 1024;
                }
                self.prg_rom[addr]
            }
            // CHR: $0000–$1FFF (PPU pattern tables)
            0x0000..=0x1FFF => self.chr_rom[addr as usize],
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            // CHR: only writable if cartridge has CHR RAM (we treat 8 KiB buffer as RAM when size is 8 KiB)
            0x0000..=0x1FFF => {
                if self.chr_rom.len() == 8 * 1024 {
                    self.chr_rom[addr as usize] = data;
                }
            }
            0x8000..=0xFFFF => {} // PRG ROM is read-only
            _ => {}
        }
    }

    /// NROM mirroring is fixed by the board; we use the value from the iNES header.
    fn mirroring(&mut self) -> Mirroring {
        self.mirroring
    }
}
