//! NES cartridge loading from iNES format (.nes files).
//!
//! Implements the [iNES](https://www.nesdev.org/wiki/INES) format: 16-byte header (magic "NES\x1A",
//! PRG size in 16 KiB units, CHR size in 8 KiB units, flags 6–7 for mapper, etc.), then PRG ROM,
//! then CHR ROM. CHR may be ROM or RAM depending on mapper. [Mapper](https://www.nesdev.org/wiki/Mapper)
//! implements CPU PRG ($8000–$FFFF) and PPU CHR ($0000–$1FFF) address decoding and bank switching.

use std::fs::File;
use std::io::Read;

use crate::cartridge::mapper::mapper::Mapper;
use crate::cartridge::mapper::mapper0::Mapper0;
use crate::cartridge::mapper::mapper1::Mapper1;
use crate::cartridge::mapper::mapper4::Mapper4;
use crate::cartridge::mapper::Mirroring;

/// Cartridge: holds PRG/CHR and the mapper that implements read/write and nametable mirroring.
/// CPU reads PRG via bus at $8000–$FFFF; PPU reads CHR at $0000–$1FFF (pattern tables).
pub struct Cartridge {
    pub mapper: Box<dyn Mapper>,
}

impl Cartridge {
    /// Load cartridge from iNES file. Header bytes 4–5 = PRG/CHR size; bytes 6–7 = mapper number
    /// (low nibble of 6 | high nibble of 7). See iNES "File format".
    pub fn load(path: &str) -> Self {
        let mut file = File::open(path).expect("Failed to open ROM");
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let prg_rom_size = data[4] as usize * 16 * 1024; // PRG ROM size in 16 KiB units
        let chr_rom_size = data[5] as usize * 8 * 1024;  // CHR ROM size in 8 KiB units (0 → 8 KiB RAM)

        let prg_start = 16;
        let prg_end = prg_start + prg_rom_size;
        let chr_start = prg_end;
        let chr_end = chr_start + chr_rom_size;

        let prg_rom = data[prg_start..prg_end].to_vec();
        let chr_rom = if chr_rom_size > 0 {
            data[chr_start..chr_end].to_vec()
        } else {
            vec![0; 8 * 1024] // No CHR ROM → 8 KiB CHR RAM (e.g. some NROM, MMC1)
        };

        // Mapper number from header bytes 6–7 (iNES). 0 = NROM, 1 = MMC1, 4 = MMC3.
        let mapper_id = (data[6] >> 4) | (data[7] & 0xF0);
        // Mirroring from iNES byte 6 bit 0: 0 = horizontal, 1 = vertical (board solder pads for NROM).
        let mirroring = if data[6] & 1 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };
        let mapper: Box<dyn Mapper> = match mapper_id {
            0 => Box::new(Mapper0::new(prg_rom, chr_rom, mirroring)),
            1 => Box::new(Mapper1::new(prg_rom)),
            4 => Box::new(Mapper4::new(prg_rom, chr_rom)),
            _ => panic!("unsupported mapper {}", mapper_id),
        };

        Self { mapper }
    }

    /// Read: PRG space ($8000–$FFFF) or CHR ($0000–$1FFF) depending on addr. Mapper dispatches.
    pub fn read(&self, addr: u16) -> u8 {
        self.mapper.read(addr)
    }

    /// Write: CHR RAM (if present) or mapper registers (e.g. MMC1 shift register). PRG ROM is R/O.
    pub fn write(&mut self, addr: u16, data: u8) {
        self.mapper.write(addr, data);
    }

    /// Notify mapper of PPU CHR read (e.g. MMC3 IRQ counter on A12 rising edge).
    pub fn on_chr_access(&mut self, addr: u16) {
        self.mapper.on_chr_access(addr);
    }

    /// Poll and clear mapper IRQ (e.g. MMC3 scanline IRQ). Returns true if IRQ was pending.
    pub fn poll_irq(&mut self) -> bool {
        self.mapper.poll_irq()
    }
}
