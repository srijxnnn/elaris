//! Memory bus and address decoding for the NES.
//!
//! Maps CPU addresses to internal RAM, PPU ($2000–$2007), APU ($4000–$4017),
//! controller ($4016), and cartridge PRG/CHR. Ticks PPU (3× CPU cycles) and APU.

use crate::apu::apu::APU;
use crate::{cartridge::cartridge::Cartridge, controller::Controller, ppu::ppu::PPU};

/// Trait for memory-mapped I/O and bus access used by the CPU.
pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn tick(&mut self, cycles: usize);
    fn poll_nmi(&mut self) -> bool;
}

/// Main NES bus: 2 KiB RAM, PPU, APU, cartridge, and controller.
/// Address decoding follows the NES memory map ($0000–$FFFF).
pub struct NesBus {
    /// Internal RAM; mirrored in $0000–$1FFF (effective 2 KiB).
    pub ram: [u8; 2048],
    pub cart: Cartridge,
    pub ppu: PPU,
    pub apu: APU,
    /// Controller port 1 ($4016); port 2 not implemented.
    pub controller: Controller,
}

impl NesBus {
    /// Create a new bus with the given cartridge.
    pub fn new(cart: Cartridge) -> Self {
        Self {
            ram: [0; 2048],
            cart,
            ppu: PPU::new(),
            apu: APU::new(),
            controller: Controller { state: 0, shift: 0 },
        }
    }

    /// True when the PPU has entered vblank; framebuffer is already filled scanline-by-scanline.
    pub fn frame_ready(&self) -> bool {
        self.ppu.frame_ready
    }

    /// Clear frame_ready after presenting (so the next frame can set it at vblank).
    pub fn clear_frame_ready(&mut self) {
        self.ppu.frame_ready = false;
    }
}

impl Bus for NesBus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // Internal RAM (mirrored 4x in 0x0000-0x1FFF)
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            // PPU: $2000–$3FFF mirrors $2000–$2007 every 8 bytes
            0x2000..=0x3FFF => {
                let r = addr & 0x2007;
                match r {
                    0x2002 => self.ppu.read_status(),
                    0x2004 => self.ppu.read_oam_data(),
                    0x2007 => self.ppu.read_data(&mut self.cart),
                    _ => 0x40, // open bus for write-only / unused
                }
            }
            // APU $4015 (status); other APU and expansion: open bus
            0x4000..=0x4014 | 0x4017..=0x401F => 0x40,
            0x4015 => self.apu.read_status(),
            0x4016 => self.controller.read(),
            // Expansion: open bus
            0x4020..=0x7FFF => 0x40,
            // Cartridge PRG ROM
            0x8000..=0xFFFF => self.cart.read(addr),
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            // Internal RAM
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = data,
            // PPU: $2000–$3FFF mirrors $2000–$2007 every 8 bytes
            0x2000..=0x3FFF => {
                let r = addr & 0x2007;
                match r {
                    0x2000 => self.ppu.write_ctrl(data),
                    0x2001 => {} // PPUMASK: not implemented (always render)
                    0x2003 => self.ppu.write_oam_addr(data),
                    0x2004 => self.ppu.write_oam_data(data),
                    0x2005 => self.ppu.write_scroll(data),
                    0x2006 => self.ppu.write_addr(data),
                    0x2007 => self.ppu.write_data(&mut self.cart, data),
                    _ => {}
                }
            }
            // APU registers
            0x4000..=0x4013 => self.apu.write(addr, data),
            0x4014 => self.ppu.oam_dma(&self.ram, data),
            0x4015 => self.apu.write(0x4015, data),
            0x4017 => self.apu.write(0x4017, data),
            0x4016 => self.controller.write(data),
            0x4018..=0x401F => {}
            0x4020..=0x7FFF => {}
            // Cartridge: mapper registers (e.g. MMC1)
            0x8000..=0xFFFF => self.cart.write(addr, data),
        }
    }

    fn tick(&mut self, cycles: usize) {
        self.apu.tick(cycles);
        // 3 PPU cycles per CPU cycle; render each scanline as it completes (real NES behavior)
        for _ in 0..(cycles * 3) {
            if let Some(scanline) = self.ppu.tick() {
                self.ppu.render_scanline(&mut self.cart, scanline);
            }
        }
    }

    fn poll_nmi(&mut self) -> bool {
        // Consume NMI if PPU triggered vblank
        if self.ppu.nmi {
            self.ppu.nmi = false;
            true
        } else {
            false
        }
    }
}
