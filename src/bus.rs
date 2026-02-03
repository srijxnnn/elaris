//! Memory bus and address decoding for the NES.
//!
//! Maps CPU addresses to RAM, PPU registers, cartridge, and controllers.

use crate::{cartridge::cartridge::Cartridge, controller::Controller, ppu::ppu::PPU};

/// Trait for memory-mapped I/O and bus access used by the CPU.
pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn tick(&mut self, cycles: usize);
    fn poll_nmi(&mut self) -> bool;
}

/// Main NES bus: RAM, PPU, cartridge, and controller.
pub struct NesBus {
    pub ram: [u8; 2048],
    pub cart: Cartridge,
    pub ppu: PPU,
    pub controller: Controller,
}

impl NesBus {
    /// Create a new bus with the given cartridge.
    pub fn new(cart: Cartridge) -> Self {
        Self {
            ram: [0; 2048],
            cart,
            ppu: PPU::new(),
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
            // PPU registers $2000-$3FFF (mirrored every 8 bytes)
            0x2000..=0x3FFF => {
                let r = addr & 0x2007;
                match r {
                    0x2002 => self.ppu.read_status(),
                    0x2004 => self.ppu.read_oam_data(),
                    0x2007 => self.ppu.read_data(&mut self.cart),
                    _ => 0x40, // open bus for write-only / unused
                }
            }
            // APU, controller 2: open bus
            0x4000..=0x4015 | 0x4017..=0x401F => 0x40,
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
            // PPU registers $2000-$3FFF (mirrored every 8 bytes)
            0x2000..=0x3FFF => {
                let r = addr & 0x2007;
                match r {
                    0x2000 => self.ppu.write_ctrl(data),
                    0x2001 => {} // PPUMASK: stub
                    0x2003 => self.ppu.write_oam_addr(data),
                    0x2004 => self.ppu.write_oam_data(data),
                    0x2005 => self.ppu.write_scroll(data),
                    0x2006 => self.ppu.write_addr(data),
                    0x2007 => self.ppu.write_data(&mut self.cart, data),
                    _ => {}
                }
            }
            // APU, expansion: no-op
            0x4000..=0x4013 | 0x4015..=0x401F => {}
            0x4014 => self.ppu.oam_dma(&self.ram, data),
            0x4016 => self.controller.write(data),
            0x4020..=0x7FFF => {}
            // Cartridge: mapper registers (e.g. MMC1)
            0x8000..=0xFFFF => self.cart.write(addr, data),
        }
    }

    fn tick(&mut self, cycles: usize) {
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
