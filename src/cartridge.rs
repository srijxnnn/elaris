use std::fs::File;
use std::io::Read;

pub struct Cartridge {
    pub prg_rom: Vec<u8>,
}

impl Cartridge {
    pub fn load(path: &str) -> Self {
        let mut file = File::open(path).expect("Failed to open ROM");
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let prg_start = 16;
        let prg_rom_size = data[4] as usize * 16 * 1024;
        let prg_end = prg_start + prg_rom_size;

        let prg_rom = data[prg_start..prg_end].to_vec();

        Self { prg_rom }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let mut addr = addr - 0x8000;
        if self.prg_rom.len() == 16 * 1024 {
            addr %= 16 * 1024;
        }
        self.prg_rom[addr as usize]
    }
}
