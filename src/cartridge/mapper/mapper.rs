//! Mapper trait: PRG/CHR memory access and mirroring.

use crate::cartridge::mapper::Mirroring;

/// Trait for NES cartridge mappers. CPU/PPU use these for all cartridge address space.
pub trait Mapper {
    /// Read from PRG ROM ($8000–$FFFF) or CHR ROM/RAM ($0000–$1FFF).
    fn read(&self, addr: u16) -> u8;
    /// Write to CHR RAM or mapper registers (PRG ROM is read-only).
    fn write(&mut self, addr: u16, data: u8);
    /// Current nametable mirroring for the PPU.
    fn mirroring(&mut self) -> Mirroring;
}
