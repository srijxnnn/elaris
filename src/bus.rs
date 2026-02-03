//! Memory bus and address decoding for the NES.
//!
//! Implements the [CPU memory map](https://www.nesdev.org/wiki/CPU_memory_map):
//!
//! | Range       | Size   | Device |
//! |-------------|--------|--------|
//! | $0000–$07FF | 2 KiB  | Internal RAM |
//! | $0800–$1FFF | mirror | Mirrors of $0000–$07FF (incomplete decode) |
//! | $2000–$2007 | 8 B    | [PPU registers](https://www.nesdev.org/wiki/PPU_registers) (mirrored every 8 bytes to $3FFF) |
//! | $4000–$4017 |        | [APU](https://www.nesdev.org/wiki/APU_registers) and I/O ($4014 = OAM DMA, $4016 = controller) |
//! | $4018–$7FFF |        | Unmapped / cartridge (e.g. PRG RAM at $6000–$7FFF) |
//! | $8000–$FFFF |        | Cartridge PRG ROM and mapper registers |
//!
//! PPU runs at 3× CPU clock; each `tick(cycles)` advances PPU by `cycles*3` and APU by `cycles`.

use crate::apu::apu::APU;
use crate::{cartridge::cartridge::Cartridge, controller::Controller, ppu::ppu::PPU};

/// Trait for memory-mapped I/O and bus access used by the CPU.
/// See NESdev "CPU memory map" for read/write behavior and open bus.
pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn tick(&mut self, cycles: usize);
    fn poll_nmi(&mut self) -> bool;
}

/// Main NES bus: 2 KiB internal RAM, PPU, APU, cartridge, controller.
/// Decoding matches the NES 2A03 address map; unmapped reads return open bus ($40 typical).
pub struct NesBus {
    /// Internal RAM; $0000–$07FF is the only physical RAM; $0800–$1FFF mirror it (addr & $07FF).
    pub ram: [u8; 2048],
    pub cart: Cartridge,
    pub ppu: PPU,
    pub apu: APU,
    /// Controller port 1 ($4016). Port 2 ($4017) not implemented. See Controller_reading.
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
            // $0000–$1FFF: Internal RAM; addresses incompletely decoded → 4 mirrors (addr & $07FF).
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            // $2000–$3FFF: PPU registers; incompletely decoded → repeat every 8 bytes. $2002=PPUSTATUS,
            // $2004=OAMDATA, $2007=PPUDATA are readable; others return open bus (e.g. $40).
            0x2000..=0x3FFF => {
                let r = addr & 0x2007;
                match r {
                    0x2002 => self.ppu.read_status(),
                    0x2004 => self.ppu.read_oam_data(),
                    0x2007 => self.ppu.read_data(&mut self.cart),
                    _ => 0x40, // Write-only or unused; open bus (Open_bus_behavior).
                }
            }
            // $4000–$4014, $4017–$401F: APU write-only / unused; open bus. $4015 is internal to CPU.
            0x4000..=0x4014 | 0x4017..=0x401F => 0x40,
            0x4015 => self.apu.read_status(),
            0x4016 => self.controller.read(),
            // $4020–$7FFF: Unmapped; available for cartridge (e.g. PRG RAM $6000–$7FFF). Open bus.
            0x4020..=0x7FFF => 0x40,
            // $8000–$FFFF: Cartridge PRG ROM (and fixed last bank for vectors $FFFA–$FFFF).
            0x8000..=0xFFFF => self.cart.read(addr),
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = data,
            // PPU: $2000=PPUCTRL, $2001=PPUMASK, $2003=OAMADDR, $2004=OAMDATA, $2005=PPUSCROLL,
            // $2006=PPUADDR, $2007=PPUDATA. Writes to $2002 (PPUSTATUS) fill internal latch only.
            0x2000..=0x3FFF => {
                let r = addr & 0x2007;
                match r {
                    0x2000 => self.ppu.write_ctrl(data),
                    0x2001 => self.ppu.write_mask(data),
                    0x2003 => self.ppu.write_oam_addr(data),
                    0x2004 => self.ppu.write_oam_data(data),
                    0x2005 => self.ppu.write_scroll(data),
                    0x2006 => self.ppu.write_addr(data),
                    0x2007 => self.ppu.write_data(&mut self.cart, data),
                    _ => {}
                }
            }
            // APU $4000–$4013 (channels), $4015 (enable/status), $4017 (frame counter). $4014=OAMDMA.
            0x4000..=0x4013 => self.apu.write(addr, data),
            0x4014 => self.ppu.oam_dma(&self.ram, data), // OAMDMA: 256-byte copy from page in data.
            0x4015 => self.apu.write(0x4015, data),
            0x4017 => self.apu.write(0x4017, data),
            0x4016 => self.controller.write(data), // Latch (bit 0): 1=strobe, then read $4016 for bits.
            0x4018..=0x401F => {}
            0x4020..=0x7FFF => {}
            // Cartridge: mapper registers (e.g. MMC1 at $8000–$FFFF by bank).
            0x8000..=0xFFFF => self.cart.write(addr, data),
        }
    }

    /// Advance PPU by 3× cycles and APU by cycles. PPU has 341 cycles per scanline; when a visible
    /// scanline (0–239) completes, we render it. See Cycle_reference_chart.
    fn tick(&mut self, cycles: usize) {
        self.apu.tick(cycles);
        for _ in 0..(cycles * 3) {
            if let Some(scanline) = self.ppu.tick() {
                self.ppu.render_scanline(&mut self.cart, scanline);
            }
        }
    }

    /// Return true once per PPU vblank if NMI is enabled (PPUCTRL bit 7). NMI fires at scanline 241,
    /// dot 1. Reading clears the NMI line so we don't re-enter. See NMI / PPU_registers.
    fn poll_nmi(&mut self) -> bool {
        if self.ppu.nmi {
            self.ppu.nmi = false;
            true
        } else {
            false
        }
    }
}
