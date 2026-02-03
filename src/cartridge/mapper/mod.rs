//! NES mappers for PRG/CHR memory mapping and nametable mirroring.
//!
//! - **Mapper 0** ([NROM](https://www.nesdev.org/wiki/NROM)): no bank switching.
//! - **Mapper 1** ([MMC1](https://www.nesdev.org/wiki/MMC1)): shift-register bank switching.
//! - **Mapper 4** ([MMC3](https://www.nesdev.org/wiki/MMC3)): bank switching, switchable mirroring, PRG RAM, scanline IRQ.
//!
//! Mirroring controls how the PPU maps the four logical nametables ($2000, $2400, $2800, $2C00) to
//! 2 KiB of internal RAM. See [PPU nametables](https://www.nesdev.org/wiki/PPU_nametables#Nametable_mirroring).

/// Nametable mirroring: Horizontal = left/right pairs share data (vertical mirroring in NESdev terms);
/// Vertical = top/bottom pairs share data (horizontal mirroring). One-screen = all four logical
/// nametables map to the same 1 KiB (lower or upper half of the 2 KiB RAM). See Mirroring.
#[derive(Clone, Copy)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    /// All four nametables use the first 1 KiB ($2000–$23FF).
    OneScreenLower,
    /// All four nametables use the second 1 KiB ($2400–$27FF).
    OneScreenUpper,
}

pub mod mapper;

pub mod mapper0;
pub mod mapper1;
pub mod mapper4;
