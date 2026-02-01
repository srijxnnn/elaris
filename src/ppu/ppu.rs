//! NES PPU (Picture Processing Unit) implementation.
//!
//! Handles vblank timing, nametable mirroring, VRAM access, and background rendering.

use crate::cartridge::{cartridge::Cartridge, mapper::Mirroring};

/// NES 2C02-style 64-color palette (0xRRGGBB). Index 0 = backdrop.
pub const NES_PALETTE_RGB: [u32; 64] = [
    0x545454, 0x001E74, 0x081090, 0x300088, 0x440064, 0x5C0030, 0x540400, 0x3C1800, 0x202A00,
    0x083A00, 0x004000, 0x003C00, 0x00302C, 0x000000, 0x000000, 0x000000, 0x989698, 0x084CC4,
    0x3032EC, 0x5C1EE4, 0x8814B0, 0xA01464, 0x982220, 0x783C00, 0x545A00, 0x287200, 0x087C00,
    0x007628, 0x006678, 0x000000, 0x000000, 0x000000, 0xECEEEC, 0x3C7EEC, 0x5C5CEC, 0x8844EC,
    0xB02CEC, 0xE028B0, 0xD83C50, 0xC45400, 0xAC7000, 0x808800, 0x409C30, 0x20A458, 0x209A88,
    0x404040, 0x000000, 0x000000, 0xECEEEC, 0xA8BCEC, 0xBCACEC, 0xD4A0EC, 0xEC94EC, 0xEC90D4,
    0xEC9CB4, 0xE4B090, 0xDCC878, 0xD4DC78, 0xB8EC98, 0xA8ECBC, 0xA0E4E4, 0xA0A0A0, 0x000000,
    0x000000,
];

/// PPU state: timing, VRAM, nametables, palettes, and framebuffer.
pub struct PPU {
    pub cycle: u16,
    pub scanline: i16,
    pub nmi: bool,
    pub vblank: bool,
    /// Set when entering vblank (scanline 241); clear after presenting the framebuffer.
    pub frame_ready: bool,
    pub ctrl: u8,
    pub addr: u16,
    pub addr_latch: bool,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub scroll_latch: bool,
    pub nametable: [u8; 0x800],
    /// Palette RAM $3F00-$3F1F (32 bytes, with NES mirroring).
    pub palette: [u8; 32],
    /// 256×240 framebuffer (0xRRGGBB per pixel). Row-major, left-to-right, top-to-bottom.
    pub framebuffer: [u32; 256 * 240],
}

impl PPU {
    /// Create PPU in initial state (pre-render scanline).
    pub fn new() -> Self {
        Self {
            cycle: 0,
            scanline: -1,
            nmi: false,
            vblank: false,
            frame_ready: false,
            ctrl: 0,
            addr: 0,
            addr_latch: false,
            scroll_x: 0,
            scroll_y: 0,
            scroll_latch: false,
            nametable: [0; 0x800],
            palette: [0; 32],
            framebuffer: [0; 256 * 240],
        }
    }

    /// Render one visible scanline into the framebuffer (background only).
    /// Called when the PPU has just finished that scanline (real NES: pixels output during the scanline).
    pub fn render_scanline(&mut self, cart: &mut Cartridge, scanline: u16) {
        let fine_x = self.scroll_x & 7;
        let fine_y = self.scroll_y & 7;
        let coarse_x = self.scroll_x >> 3;
        let coarse_y = self.scroll_y >> 3;
        let nametable_base = (self.ctrl & 3) as u16;
        let bg_pattern_base = if self.ctrl & 0x10 != 0 {
            0x1000u16
        } else {
            0x0000
        };
        let mirroring = cart.mapper.mirroring();
        let y = scanline;

        for x in 0..256u16 {
            let total_x = (x as u32 + fine_x as u32 + (coarse_x as u32) * 8) % 512;
            let total_y = (y as u32 + fine_y as u32 + (coarse_y as u32) * 8) % 480;
            let tile_x = (total_x / 8) as u16;
            let tile_y = (total_y / 8) as u16;

            let nt_x = tile_x / 32;
            let nt_y = tile_y / 30;
            let nt_phys = match mirroring {
                Mirroring::Horizontal => (nametable_base & 1) ^ nt_x,
                Mirroring::Vertical => (nametable_base >> 1) ^ nt_y,
            };
            let tile_x_in_nt = tile_x % 32;
            let tile_y_in_nt = tile_y % 30;

            let nt_index = (nt_phys * 0x400) + tile_y_in_nt * 32 + tile_x_in_nt;
            let tile_id = self.nametable[nt_index as usize];

            let attr_index =
                (nt_phys * 0x400) + 0x3C0 + (tile_y_in_nt / 4) * 8 + (tile_x_in_nt / 4);
            let attr_byte = self.nametable[attr_index as usize];
            let shift = ((tile_y_in_nt & 1) << 2) | ((tile_x_in_nt & 1) << 1);
            let palette_bank = (attr_byte >> shift) & 3;

            let px_in_tile = (total_x % 8) as u16;
            let py_in_tile = (total_y % 8) as u16;
            let tile_addr = bg_pattern_base + (tile_id as u16) * 16;
            let row_lo = cart.read(tile_addr + py_in_tile);
            let row_hi = cart.read(tile_addr + py_in_tile + 8);
            let bit = 7 - (px_in_tile % 8);
            let low = (row_lo >> bit) & 1;
            let high = (row_hi >> bit) & 1;
            let pixel_value = (high << 1) | low;

            let palette_idx = 0x3F00 + (palette_bank as u16) * 4 + (pixel_value as u16);
            let color_idx = self.palette[Self::palette_index(palette_idx)] as usize;
            let rgb = NES_PALETTE_RGB[color_idx & 0x3F];

            self.framebuffer[(y as usize) * 256 + (x as usize)] = rgb;
        }
    }

    /// Resolve PPU palette address $3F00-$3F1F (and $3F20-$3FFF mirrors) to palette index.
    /// $3F10, $3F14, $3F18, $3F1C mirror $3F00 (background color).
    fn palette_index(addr: u16) -> usize {
        let i = (addr & 0x1F) as usize;
        if i == 16 || i == 20 || i == 24 || i == 28 {
            0
        } else {
            i
        }
    }

    /// Advance PPU by one cycle; update vblank/NMI timing.
    /// Returns `Some(scanline)` when a visible scanline (0..240) has just been completed,
    /// so the bus can render that scanline (real NES: pixels were output during the scanline).
    pub fn tick(&mut self) -> Option<u16> {
        self.cycle += 1;

        // Start of vblank (scanline 241, cycle 1)
        if self.scanline == 241 && self.cycle == 1 {
            self.vblank = true;
            self.frame_ready = true;
            if self.ctrl & 0x80 != 0 {
                self.nmi = true;
            }
        }

        // Clear vblank at end of pre-render
        if self.scanline == -1 && self.cycle == 1 {
            self.vblank = false;
        }

        // End of scanline (341 cycles per scanline)
        let mut completed_scanline = None;
        if self.cycle == 341 {
            // Just finished this scanline; if visible, caller should render it
            if self.scanline >= 0 && self.scanline < 240 {
                completed_scanline = Some(self.scanline as u16);
            }
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline == 261 {
                self.scanline = -1;
            }
        }
        completed_scanline
    }

    /// Read PPUSTATUS ($2002); clears vblank, addr latch, scroll latch.
    pub fn read_status(&mut self) -> u8 {
        let mut status = 0u8;

        if self.vblank {
            status |= 0x80;
        }

        self.vblank = false;
        self.nmi = false;
        self.addr_latch = false;
        self.scroll_latch = false;

        status
    }

    /// Write PPUCTRL ($2000).
    pub fn write_ctrl(&mut self, data: u8) {
        self.ctrl = data;
    }

    /// Write PPUADDR ($2006); 16-bit register written as two bytes.
    pub fn write_addr(&mut self, data: u8) {
        if !self.addr_latch {
            self.addr = (data as u16) << 8;
            self.addr_latch = true;
        } else {
            self.addr |= data as u16;
            self.addr_latch = false;
        }
    }

    /// Read PPUDATA ($2007); auto-increments VRAM address.
    pub fn read_data(&mut self, cart: &mut Cartridge) -> u8 {
        let addr = self.addr & 0x3FFF;

        let data = match addr {
            // CHR: pattern tables
            0x0000..=0x1FFF => cart.read(addr),

            // Nametables (with mirroring)
            0x2000..=0x2FFF => {
                let mirroring = cart.mapper.mirroring();
                let index = Self::map_nametable_addr(addr, mirroring);
                self.nametable[index as usize]
            }

            // Mirrors of $2000–$2EFF
            0x3000..=0x3EFF => {
                let mirrored = addr - 0x1000;
                let mirroring = cart.mapper.mirroring();
                let index = Self::map_nametable_addr(mirrored, mirroring);
                self.nametable[index as usize]
            }

            // Palette RAM $3F00-$3F1F and $3F20-$3FFF mirrors
            0x3F00..=0x3FFF => self.palette[Self::palette_index(addr)],
            _ => 0,
        };

        // Increment by 32 if PPUCTRL bit 2 set, else 1
        let inc = if self.ctrl & 0x04 != 0 { 32 } else { 1 };
        self.addr = self.addr.wrapping_add(inc);
        data
    }

    /// Write PPUDATA ($2007); auto-increments VRAM address.
    pub fn write_data(&mut self, cart: &mut Cartridge, data: u8) {
        let addr = self.addr & 0x3FFF;

        match addr {
            // CHR RAM only (writes to ROM ignored by mapper)
            0x0000..=0x1FFF => {
                cart.write(addr, data);
            }

            // Nametables
            0x2000..=0x2FFF => {
                let mirroring = cart.mapper.mirroring();
                let index = Self::map_nametable_addr(addr, mirroring);
                self.nametable[index as usize] = data;
            }

            // Mirrors
            0x3000..=0x3EFF => {
                let mirrored = addr - 0x1000;
                let mirroring = cart.mapper.mirroring();
                let index = Self::map_nametable_addr(mirrored, mirroring);
                self.nametable[index as usize] = data;
            }

            // Palette RAM $3F00-$3F1F and $3F20-$3FFF mirrors (upper 2 bits of data ignored on real NES)
            0x3F00..=0x3FFF => self.palette[Self::palette_index(addr)] = data & 0x3F,
            _ => {}
        }

        // Increment by 32 if PPUCTRL bit 2 set, else 1
        let inc = if self.ctrl & 0x04 != 0 { 32 } else { 1 };
        self.addr = self.addr.wrapping_add(inc);
    }

    /// Write PPUSCROLL ($2005); two writes for X and Y scroll.
    pub fn write_scroll(&mut self, data: u8) {
        if !self.scroll_latch {
            self.scroll_x = data;
            self.scroll_latch = true;
        } else {
            self.scroll_y = data;
            self.scroll_latch = false;
        }
    }

    /// Map PPU nametable address to internal index based on mirroring mode.
    pub fn map_nametable_addr(addr: u16, mirroring: Mirroring) -> u16 {
        let addr = (addr - 0x2000) & 0xfff;
        let table = addr / 0x400;
        let offset = addr & 0x3ff;

        match mirroring {
            Mirroring::Vertical => match table {
                0 | 1 => offset,
                2 | 3 => offset + 0x400,
                _ => unreachable!(),
            },
            Mirroring::Horizontal => match table {
                0 | 2 => offset,
                1 | 3 => offset + 0x400,
                _ => unreachable!(),
            },
        }
    }
}
