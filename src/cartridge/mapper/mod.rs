//! NES mappers for PRG/CHR memory mapping.
//!
//! Mapper0 (NROM), Mapper1 (MMC1). Each mapper provides read/write and mirroring.

/// Nametable mirroring mode for the PPU (how $2000–$2FFF maps to internal nametables).
#[derive(Clone, Copy)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

pub mod mapper;

pub mod mapper0;
pub mod mapper1;
