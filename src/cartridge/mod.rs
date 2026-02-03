//! NES cartridge loading and mapper support.
//!
//! - **cartridge**: Loads iNES (.nes) files, holds PRG/CHR and mapper.
//! - **mapper**: NROM (0), MMC1 (1), MMC3 (4); PRG/CHR bank switching and nametable mirroring.

pub mod cartridge;
pub mod mapper;