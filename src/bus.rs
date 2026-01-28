use crate::cartridge::Cartridge;

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

pub struct NesBus {
    pub ram: [u8; 2048],
    pub cart: Cartridge,
}

impl NesBus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            ram: [0; 2048],
            cart,
        }
    }
}

impl Bus for NesBus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x8000..=0xFFFF => self.cart.read(addr),
            _ => 0, // PPU/APU ignored for nestest
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram[(addr & 0x07FF) as usize] = data;
            }
            _ => {} // Ignore writes to ROM/PPU
        }
    }
}
