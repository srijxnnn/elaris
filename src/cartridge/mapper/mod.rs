//! NES mappers for PRG/CHR memory mapping.
//!
//! Mapper0 (NROM), Mapper1 (MMC1), and common types.

/// Nametable mirroring mode for PPU.
#[derive(Clone, Copy)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

pub mod mapper;

pub mod mapper0;
pub mod mapper1;
