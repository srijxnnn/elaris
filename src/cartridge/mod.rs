//! NES cartridge loading and mapper support.
//!
//! Parses iNES format ROMs and delegates memory access to mappers.

pub mod cartridge;
pub mod mapper;