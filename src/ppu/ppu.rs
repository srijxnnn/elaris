//! NES PPU (Picture Processing Unit) implementation.
//!
//! Emulates the [PPU](https://www.nesdev.org/wiki/PPU) (2C02): [PPU registers](https://www.nesdev.org/wiki/PPU_registers)
//! $2000–$2007 (mirrored every 8 bytes to $3FFF), [PPU memory map](https://www.nesdev.org/wiki/PPU_memory_map)
//! (pattern tables, nametables, palette), [OAM](https://www.nesdev.org/wiki/PPU_OAM), and rendering.
//!
//! ## Timing
//!
//! 341 PPU cycles per scanline; 262 scanlines per frame (0–239 visible, 240 post-render, 241 vblank,
//! 261 pre-render). VBlank NMI at scanline 241, cycle 1. PPU runs at 3× CPU clock.
//!
//! ## References
//!
//! - [PPU scrolling](https://www.nesdev.org/wiki/PPU_scrolling) (internal v, t, x, w)
//! - [PPU rendering](https://www.nesdev.org/wiki/PPU_rendering), [Nametables](https://www.nesdev.org/wiki/PPU_nametables)
//! - [Sprite 0 hit](https://www.nesdev.org/wiki/PPU_OAM#Sprite_zero_hits), [Sprite overflow](https://www.nesdev.org/wiki/PPU_sprite_evaluation#Sprite_overflow_bug)

use crate::cartridge::{cartridge::Cartridge, mapper::Mirroring};

/// NES 2C02 64-color RGB palette (0xRRGGBB). Used to convert 6-bit palette indices to display.
/// Index 0 is the backdrop color ($3F00). See PPU_palettes.
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

/// OAM (Object Attribute Memory): 64 sprites × 4 bytes. Each entry: Y, tile index, attributes, X.
/// See PPU_OAM (byte 0=Y, 1=tile, 2=attr, 3=X).
pub const OAM_LEN: usize = 256;

/// PPU state: cycle (0–340) and scanline (-1=pre-render, 0–239=visible, 241=vblank start), internal
/// registers, nametable RAM (2 KiB), palette (32 bytes $3F00–$3F1F), OAM, and framebuffer.
pub struct PPU {
    pub cycle: u16,
    pub scanline: i16,
    pub nmi: bool,
    pub vblank: bool,
    /// True when we've entered vblank (scanline 241); host clears after presenting frame.
    pub frame_ready: bool,
    /// PPUCTRL ($2000): NMI enable, sprite size, bg/sprite pattern table, increment, nametable.
    pub ctrl: u8,
    /// PPUMASK ($2001): grayscale (0), show bg/sprite left 8 (1,2), show bg/sprite (3,4), emphasis (5–7).
    pub mask: u8,
    /// Current VRAM address (PPUADDR $2006; 14-bit). Shared with scroll internally.
    pub addr: u16,
    pub addr_latch: bool,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub scroll_latch: bool,
    /// Nametable RAM: 2 KiB for $2000–$2FFF (mirroring applied per cartridge). PPU_nametables.
    pub nametable: [u8; 0x800],
    /// Palette RAM: 32 bytes ($3F00–$3F1F); $3F10/14/18/1C mirror $3F00. PPU_palettes.
    pub palette: [u8; 32],
    /// OAM: 256 bytes. Filled via OAMDATA ($2003/$2004) or OAMDMA ($4014).
    pub oam: [u8; OAM_LEN],
    pub oam_addr: u8,
    /// Sprite 0 hit flag (PPUSTATUS bit 6). Set when sprite 0 overlaps background; clear on $2002 read.
    pub sprite_0_hit: bool,
    /// Sprite overflow flag (PPUSTATUS bit 5). Buggy on hardware; clear on $2002 read.
    pub sprite_overflow: bool,
    /// 256×240 framebuffer (one u32 0xRRGGBB per pixel). Row-major, scanline 0 = top.
    pub framebuffer: [u32; 256 * 240],
}

impl PPU {
    /// Create PPU in initial state (pre-render scanline -1, cycle 0).
    pub fn new() -> Self {
        Self {
            cycle: 0,
            scanline: -1,
            nmi: false,
            vblank: false,
            frame_ready: false,
            ctrl: 0,
            mask: 0,
            addr: 0,
            addr_latch: false,
            scroll_x: 0,
            scroll_y: 0,
            scroll_latch: false,
            nametable: [0; 0x800],
            palette: [0; 32],
            oam: [0; OAM_LEN],
            oam_addr: 0,
            sprite_0_hit: false,
            sprite_overflow: false,
            framebuffer: [0; 256 * 240],
        }
    }

    /// Apply PPUMASK: grayscale (bit 0) and color emphasis (bits 5–7). See PPUMASK "Color control".
    fn apply_display_mask(&self, rgb: u32) -> u32 {
        let r = ((rgb >> 16) & 0xFF) as u32;
        let g = ((rgb >> 8) & 0xFF) as u32;
        let b = (rgb & 0xFF) as u32;
        let rgb = if self.mask & 0x01 != 0 {
            let gray = (r + g + b) / 3;
            (gray << 16) | (gray << 8) | gray
        } else {
            rgb
        };
        self.apply_emphasis(rgb)
    }

    /// Dim channels not emphasized by PPUMASK bits 5 (R), 6 (G), 7 (B).
    fn apply_emphasis(&self, rgb: u32) -> u32 {
        let m = self.mask;
        let mut r = ((rgb >> 16) & 0xFF) as u32;
        let mut g = ((rgb >> 8) & 0xFF) as u32;
        let mut b = (rgb & 0xFF) as u32;
        if m & 0x20 == 0 {
            r = r * 2 / 3;
        }
        if m & 0x40 == 0 {
            g = g * 2 / 3;
        }
        if m & 0x80 == 0 {
            b = b * 2 / 3;
        }
        (r << 16) | (g << 8) | b
    }

    /// Render one visible scanline (0–239) into the framebuffer: background from nametable + scroll,
    /// then up to 8 sprites (priority: lower OAM index on top). Sprite 0 hit and overflow set here.
    /// Called when the PPU cycle counter has just completed that scanline (341 cycles). PPU_rendering.
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

        let show_bg = self.mask & 0x08 != 0;
        let show_sprites = self.mask & 0x10 != 0;
        let show_bg_left = self.mask & 0x02 != 0;
        let show_sprites_left = self.mask & 0x04 != 0;

        let backdrop_idx = self.palette[0] as usize & 0x3F;
        let backdrop_rgb = NES_PALETTE_RGB[backdrop_idx];

        // Background pixel values (0-3) per x for sprite 0 hit and priority. 0 = transparent.
        let mut bg_pixel: [u8; 256] = [0; 256];

        for x in 0..256u16 {
            let total_x = (x as u32 + fine_x as u32 + (coarse_x as u32) * 8) % 512;
            let total_y = (y as u32 + fine_y as u32 + (coarse_y as u32) * 8) % 480;
            let tile_x = (total_x / 8) as u16;
            let tile_y = (total_y / 8) as u16;

            let nt_x = tile_x / 32;
            let nt_y = tile_y / 30;

            let base_nt_x = nametable_base & 1;
            let base_nt_y = (nametable_base >> 1) & 1;

            let logical_nt = (base_nt_x ^ nt_x) + ((base_nt_y ^ nt_y) << 1);
            let nt_phys = match mirroring {
                Mirroring::Vertical => logical_nt & 1,
                Mirroring::Horizontal => logical_nt >> 1,
                Mirroring::OneScreenLower => 0,
                Mirroring::OneScreenUpper => 1,
            };
            let tile_x_in_nt = tile_x % 32;
            let tile_y_in_nt = tile_y % 30;

            let nt_index = (nt_phys * 0x400) + tile_y_in_nt * 32 + tile_x_in_nt;
            let tile_id = self.nametable[nt_index as usize];

            let attr_index =
                (nt_phys * 0x400) + 0x3C0 + (tile_y_in_nt / 4) * 8 + (tile_x_in_nt / 4);
            let attr_byte = self.nametable[attr_index as usize];
            let shift = ((tile_y_in_nt & 2) << 1) | (tile_x_in_nt & 2);
            let palette_bank = (attr_byte >> shift) & 3;

            let px_in_tile = (total_x % 8) as u16;
            let py_in_tile = (total_y % 8) as u16;
            let tile_addr = bg_pattern_base + (tile_id as u16) * 16;
            cart.on_chr_access(tile_addr + py_in_tile);
            let row_lo = cart.read(tile_addr + py_in_tile);
            cart.on_chr_access(tile_addr + py_in_tile + 8);
            let row_hi = cart.read(tile_addr + py_in_tile + 8);
            let bit = 7 - (px_in_tile % 8);
            let low = (row_lo >> bit) & 1;
            let high = (row_hi >> bit) & 1;
            let pixel_value = (high << 1) | low;

            bg_pixel[x as usize] = pixel_value;

            let rgb = if show_bg && (x >= 8 || show_bg_left) && pixel_value != 0 {
                let palette_idx = 0x3F00 + (palette_bank as u16) * 4 + (pixel_value as u16);
                let color_idx = self.palette[Self::palette_index(palette_idx)] as usize;
                NES_PALETTE_RGB[color_idx & 0x3F]
            } else {
                backdrop_rgb
            };
            self.framebuffer[(y as usize) * 256 + (x as usize)] = self.apply_display_mask(rgb);
        }

        // Sprite evaluation: find up to 8 sprites on this scanline (lower OAM index = higher priority).
        let sprite_height = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
        let sprite_pattern_base = if self.ctrl & 0x08 != 0 {
            0x1000u16
        } else {
            0x0000
        };

        #[derive(Clone, Copy)]
        struct SpriteSlot {
            oam_index: u8,
            y_offset: u8,
            y: u8,
            tile: u8,
            attr: u8,
            x: u8,
        }

        let mut slots: [Option<SpriteSlot>; 8] = [None; 8];
        let mut slot_count = 0u8;

        for i in 0..64u8 {
            let base = (i as usize) * 4;
            let oam_y = self.oam[base];
            let oam_tile = self.oam[base + 1];
            let oam_attr = self.oam[base + 2];
            let oam_x = self.oam[base + 3];

            let y_lo = oam_y as u16;
            let y_hi = y_lo + sprite_height;
            if scanline >= y_lo && scanline < y_hi {
                if slot_count < 8 {
                    let y_offset = (scanline - y_lo) as u8;
                    slots[slot_count as usize] = Some(SpriteSlot {
                        oam_index: i,
                        y_offset,
                        y: oam_y,
                        tile: oam_tile,
                        attr: oam_attr,
                        x: oam_x,
                    });
                    slot_count += 1;
                } else {
                    self.sprite_overflow = true;
                }
            }
        }

        // Draw sprites back-to-front (highest OAM index first) so lower-index sprites appear on top.
        for s in (0..slot_count).rev() {
            let slot = slots[s as usize].unwrap();
            let flip_v = slot.attr & 0x80 != 0;
            let flip_h = slot.attr & 0x40 != 0;
            let behind_bg = slot.attr & 0x20 != 0;
            let palette_bank = (slot.attr & 3) as u16;
            let palette_base = 0x3F10 + palette_bank * 4;

            let row_in_sprite = if flip_v {
                (sprite_height - 1) as u8 - slot.y_offset
            } else {
                slot.y_offset
            };

            let (tile_addr, row_in_tile) = if sprite_height == 8 {
                let addr = sprite_pattern_base + (slot.tile as u16) * 16;
                (addr, row_in_sprite)
            } else {
                let table = (slot.tile & 1) as u16 * 0x1000;
                let tile_8 = (slot.tile & 0xFE) as u16;
                let (tile_idx, row) = if row_in_sprite < 8 {
                    (tile_8, row_in_sprite)
                } else {
                    (tile_8 + 1, row_in_sprite - 8)
                };
                (table + tile_idx * 16, row)
            };

            cart.on_chr_access(tile_addr + row_in_tile as u16);
            let row_lo = cart.read(tile_addr + row_in_tile as u16);
            cart.on_chr_access(tile_addr + row_in_tile as u16 + 8);
            let row_hi = cart.read(tile_addr + row_in_tile as u16 + 8);

            if !show_sprites {
                continue;
            }
            for px in 0..8u16 {
                let col = if flip_h { 7 - px } else { px };
                let bit = 7 - col;
                let low = (row_lo >> bit) & 1;
                let high = (row_hi >> bit) & 1;
                let pixel_value = (high << 1) | low;

                let screen_x = (slot.x as i16 + px as i16) as usize;
                if screen_x >= 256 {
                    continue;
                }
                if screen_x < 8 && !show_sprites_left {
                    continue;
                }
                let idx = (y as usize) * 256 + screen_x;
                let bg_val = bg_pixel[screen_x];

                if pixel_value == 0 {
                    continue;
                }
                if slot.oam_index == 0 && bg_val != 0 {
                    self.sprite_0_hit = true;
                }
                if behind_bg && bg_val != 0 {
                    continue;
                }

                let palette_idx = palette_base + pixel_value as u16;
                let color_idx = self.palette[Self::palette_index(palette_idx)] as usize;
                let rgb = NES_PALETTE_RGB[color_idx & 0x3F];
                self.framebuffer[idx] = self.apply_display_mask(rgb);
            }
        }
    }

    /// Resolve PPU palette address $3F00–$3F1F (and $3F20–$3FFF mirrors) to 32-byte index.
    /// Addresses $3F10, $3F14, $3F18, $3F1C mirror $3F00 (background color).
    fn palette_index(addr: u16) -> usize {
        let i = (addr & 0x1F) as usize;
        if i == 16 || i == 20 || i == 24 || i == 28 {
            0
        } else {
            i
        }
    }

    /// Advance PPU by one cycle. 341 cycles per scanline; at cycle 1 of scanline 241 set vblank and
    /// optionally NMI. Returns Some(scanline) when a visible scanline (0–239) has just completed
    /// (cycle 341), so the bus can render it. See PPU_rendering, Cycle_reference_chart.
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

    /// Read PPUSTATUS ($2002): bits 7=vblank, 6=sprite 0 hit, 5=sprite overflow; lower bits open bus.
    /// Side effect: clears vblank/sprite flags and the w (write latch) for PPUSCROLL/PPUADDR.
    pub fn read_status(&mut self) -> u8 {
        let mut status = 0u8;

        if self.vblank {
            status |= 0x80;
        }
        if self.sprite_0_hit {
            status |= 0x40;
        }
        if self.sprite_overflow {
            status |= 0x20;
        }

        self.vblank = false;
        self.nmi = false;
        self.sprite_0_hit = false;
        self.sprite_overflow = false;
        self.addr_latch = false;
        self.scroll_latch = false;

        status
    }

    /// Write OAMADDR ($2003).
    pub fn write_oam_addr(&mut self, data: u8) {
        self.oam_addr = data;
    }

    /// Read OAMDATA ($2004); returns OAM byte at current OAMADDR (read does not increment on real NES).
    pub fn read_oam_data(&mut self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    /// Write OAMDATA ($2004); writes OAM and increments OAMADDR.
    pub fn write_oam_data(&mut self, data: u8) {
        self.oam[self.oam_addr as usize] = data;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    /// OAMDMA ($4014): copy 256 bytes from CPU RAM page `page` (high byte of address, e.g. $02 →
    /// $0200–$02FF) into OAM. Takes 513–514 cycles; we copy in one go. OAMADDR should be 0 first.
    pub fn oam_dma(&mut self, ram: &[u8; 2048], page: u8) {
        let start = ((page as u16) << 8) as usize % 2048;
        for i in 0..256 {
            self.oam[i] = ram[(start + i) % 2048];
        }
    }

    /// Write PPUCTRL ($2000).
    pub fn write_ctrl(&mut self, data: u8) {
        self.ctrl = data;
    }

    /// Write PPUMASK ($2001).
    pub fn write_mask(&mut self, data: u8) {
        self.mask = data;
    }

    /// Write PPUADDR ($2006): two-byte write for 16-bit VRAM address (high then low).
    pub fn write_addr(&mut self, data: u8) {
        if !self.addr_latch {
            self.addr = (data as u16) << 8;
            self.addr_latch = true;
        } else {
            self.addr |= data as u16;
            self.addr_latch = false;
        }
    }

    /// Read PPUDATA ($2007): return byte at current VRAM address; then increment addr by 1 or 32
    /// (PPUCTRL bit 2). Palette $3F00–$3F1F returns immediately; other reads use internal buffer.
    pub fn read_data(&mut self, cart: &mut Cartridge) -> u8 {
        let addr = self.addr & 0x3FFF;

        let data = match addr {
            // CHR: pattern tables (notify mapper for e.g. MMC3 IRQ A12 tracking)
            0x0000..=0x1FFF => {
                cart.on_chr_access(addr);
                cart.read(addr)
            }

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

    /// Write PPUDATA ($2007): writes VRAM at current address, then increments (by 1 or 32 per PPUCTRL).
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

    /// Write PPUSCROLL ($2005): two-byte write. First = X scroll (fine + coarse low 8); second = Y.
    /// Bit 8 of X/Y come from PPUCTRL bits 0–1. Shared w register with PPUADDR; read $2002 to reset.
    pub fn write_scroll(&mut self, data: u8) {
        if !self.scroll_latch {
            self.scroll_x = data;
            self.scroll_latch = true;
        } else {
            self.scroll_y = data;
            self.scroll_latch = false;
        }
    }

    /// Map PPU nametable VRAM address ($2000–$2FFF) to internal 2 KiB index using mirroring.
    pub fn map_nametable_addr(addr: u16, mirroring: Mirroring) -> u16 {
        let addr = (addr - 0x2000) & 0xfff;
        let table = addr / 0x400;
        let offset = addr & 0x3ff;

        match mirroring {
            Mirroring::Vertical => match table {
                0 | 2 => offset,
                1 | 3 => offset + 0x400,
                _ => unreachable!(),
            },
            Mirroring::Horizontal => match table {
                0 | 1 => offset,
                2 | 3 => offset + 0x400,
                _ => unreachable!(),
            },
            Mirroring::OneScreenLower => offset,
            Mirroring::OneScreenUpper => offset + 0x400,
        }
    }
}
