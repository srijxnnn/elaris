//! Mapper trait: PRG/CHR memory access and nametable mirroring.
//!
//! [Mapper](https://www.nesdev.org/wiki/Mapper) is the cartridge logic that maps CPU PRG ($8000–$FFFF)
//! and PPU CHR ($0000–$1FFF) to ROM/RAM and handles bank switching. Mirroring tells the PPU how
//! to map nametable addresses ($2000–$2FFF) to the internal 2 KiB nametable RAM. See Mirroring.

use crate::cartridge::mapper::Mirroring;

/// Trait for NES cartridge mappers. Bus/cartridge call read/write; PPU calls mirroring() for
/// nametable layout (horizontal, vertical, or one-screen).
pub trait Mapper {
    /// Read: PRG at $8000–$FFFF (CPU), CHR at $0000–$1FFF (PPU pattern tables). Address is the
    /// full 16-bit address in the respective space (PPU passes 14-bit $0000–$3FFF; we use low 13 for CHR).
    fn read(&self, addr: u16) -> u8;
    /// Write: CHR RAM (if present) or mapper registers. PRG ROM is read-only; writes to PRG often
    /// control bank switching (e.g. MMC1 shift register).
    fn write(&mut self, addr: u16, data: u8);
    /// Current nametable mirroring: horizontal (vertical mirroring), vertical (horizontal
    /// mirroring), or one-screen. PPU uses this to map $2000–$2FFF to 2 KiB. See PPU_nametables.
    fn mirroring(&mut self) -> Mirroring;
}
