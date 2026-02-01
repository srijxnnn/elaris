//! Mapper trait: PRG/CHR memory access and mirroring.

use crate::cartridge::mapper::Mirroring;

/// Trait for NES cartridge mappers (PRG/CHR bank switching).
pub trait Mapper {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn mirroring(&mut self) -> Mirroring;
}
