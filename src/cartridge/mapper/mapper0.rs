//! Mapper 0 (NROM): no bank switching, 16/32KB PRG, 8KB CHR.

use crate::cartridge::mapper::{Mirroring, mapper::Mapper};

/// NROM mapper: fixed PRG and CHR, optionally 16KB PRG mirror.
pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
}

impl Mapper0 {
    /// Create Mapper0 with given PRG and CHR ROM.
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Self { prg_rom, chr_rom }
    }
}

impl Mapper for Mapper0 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            // PRG ROM: $8000-$FFFF, mirror if 16KB
            0x8000..=0xFFFF => {
                let mut addr = (addr - 0x8000) as usize;
                if self.prg_rom.len() == 16 * 1024 {
                    addr %= 16 * 1024;
                }
                self.prg_rom[addr]
            }
            // CHR ROM: $0000-$1FFF
            0x0000..=0x1FFF => self.chr_rom[addr as usize],
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            // CHR RAM only if cartridge uses 8KB CHR RAM
            0x0000..=0x1FFF => {
                if self.chr_rom.len() == 8 * 1024 {
                    self.chr_rom[addr as usize] = data;
                }
            }
            0x8000..=0xFFFF => {} // PRG ROM: no writes
            _ => {}
        }
    }

    fn mirroring(&mut self) -> Mirroring {
        Mirroring::Horizontal
    }
}
