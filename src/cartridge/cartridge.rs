//! NES cartridge loading from iNES format (.nes files).
//!
//! Header: 16 bytes (magic, PRG size × 16 KiB, CHR size × 8 KiB, flags, mapper, etc.).
//! Then PRG ROM, then CHR ROM (or CHR RAM for some mappers).

use std::fs::File;
use std::io::Read;

use crate::cartridge::mapper::mapper::Mapper;
use crate::cartridge::mapper::mapper0::Mapper0;
use crate::cartridge::mapper::mapper1::Mapper1;

/// Cartridge: holds the mapper that implements PRG ($8000–$FFFF) and CHR ($0000–$1FFF) access.
pub struct Cartridge {
    pub mapper: Box<dyn Mapper>,
}

impl Cartridge {
    /// Load cartridge from iNES format file.
    pub fn load(path: &str) -> Self {
        let mut file = File::open(path).expect("Failed to open ROM");
        // iNES header: 16 bytes, then PRG ROM, then CHR ROM
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let prg_rom_size = data[4] as usize * 16 * 1024; // Header byte 4: PRG ROM size in 16KB units
        let chr_rom_size = data[5] as usize * 8 * 1024; // Header byte 5: CHR ROM size in 8KB units

        let prg_start = 16;
        let prg_end = prg_start + prg_rom_size;
        let chr_start = prg_end;
        let chr_end = chr_start + chr_rom_size;

        let prg_rom = data[prg_start..prg_end].to_vec();
        let chr_rom = if chr_rom_size > 0 {
            data[chr_start..chr_end].to_vec()
        } else {
            vec![0; 8 * 1024]
        };

        // Mapper number: low nibble of byte 6, high nibble of byte 7
        let mapper_id = (data[6] >> 4) | (data[7] & 0xF0);
        let mapper: Box<dyn Mapper> = match mapper_id {
            0 => Box::new(Mapper0::new(prg_rom, chr_rom)),
            1 => Box::new(Mapper1::new(prg_rom)),
            _ => panic!("unsupported mapper {}", mapper_id),
        };

        Self { mapper }
    }

    /// Read from PRG or CHR depending on address.
    pub fn read(&self, addr: u16) -> u8 {
        self.mapper.read(addr)
    }

    /// Write to CHR RAM or mapper registers.
    pub fn write(&mut self, addr: u16, data: u8) {
        self.mapper.write(addr, data);
    }
}
