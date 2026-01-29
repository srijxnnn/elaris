use crate::{cartridge::Cartridge, ppu::ppu::PPU};

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn tick(&mut self, cycles: usize);
    fn poll_nmi(&mut self) -> bool;
    fn read_ppu_status(&mut self) -> u8;
    fn write_ppu_ctrl(&mut self, data: u8);
}

pub struct NesBus {
    pub ram: [u8; 2048],
    pub cart: Cartridge,
    pub ppu: PPU,
}

impl NesBus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            ram: [0; 2048],
            cart,
            ppu: PPU::new(),
        }
    }
}

impl Bus for NesBus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x2002 => self.read_ppu_status(),
            0x8000..=0xFFFF => self.cart.read(addr),
            _ => 0, // PPU/APU ignored for nestest
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram[(addr & 0x07FF) as usize] = data;
            }
            0x2000 => self.write_ppu_ctrl(data),
            _ => {} // Ignore writes to ROM/PPU
        }
    }

    fn tick(&mut self, cycles: usize) {
        for _ in 0..(cycles * 3) {
            self.ppu.tick();
        }
    }

    fn poll_nmi(&mut self) -> bool {
        if self.ppu.nmi {
            self.ppu.nmi = false;
            true
        } else {
            false
        }
    }

    fn read_ppu_status(&mut self) -> u8 {
        let mut status = 0u8;

        if self.ppu.vblank {
            status |= 0x80;
        }

        self.ppu.vblank = false;
        self.ppu.nmi = false;

        status
    }

    fn write_ppu_ctrl(&mut self, data: u8) {
        self.ppu.ctrl = data;
    }
}
