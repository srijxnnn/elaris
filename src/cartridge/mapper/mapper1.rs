//! Mapper 1 (MMC1): bank switching via 5-bit shift register.
//!
//! [MMC1](https://www.nesdev.org/wiki/MMC1): writes to $8000–$9FFF (control), $A000–$BFFF (CHR0),
//! $C000–$DFFF (CHR1), $E000–$FFFF (PRG bank). Any write with bit 7 set resets the shift register.
//! Otherwise, bit 0 is shifted in (LSB first); after 5 writes, the value is latched to the selected
//! register. Control (bits 0–1) = mirroring; bits 2–3 = PRG mode; bit 4 = CHR mode. We implement
//! PRG banking only; CHR banking is omitted (single bank or RAM as needed).

use crate::cartridge::mapper::{Mirroring, mapper::Mapper};

/// MMC1 state: 5-bit shift register, control byte (mirroring + PRG/CHR mode), PRG bank select.
pub struct Mapper1 {
    prg_rom: Vec<u8>,
    shift_reg: u8,
    shift_count: u8,
    control: u8,
    prg_bank: u8,
}

impl Mapper1 {
    /// Create MMC1 with PRG ROM. Control defaults to $0C (PRG mode 3: $8000 switchable, $C000 fixed last).
    pub fn new(prg_rom: Vec<u8>) -> Self {
        Self {
            prg_rom,
            shift_reg: 0,
            shift_count: 0,
            control: 0x0C,
            prg_bank: 0,
        }
    }

    /// PRG bank mode from control bits 2–3: 0/1 = 32 KiB mode; 2 = $8000 fixed first, $C000 switchable; 3 = $8000 switchable, $C000 fixed last.
    fn prg_bank_mode(&self) -> u8 {
        (self.control >> 2) & 0b11
    }

    fn prg_bank_count(&self) -> usize {
        self.prg_rom.len() / 0x4000
    }
}

impl Mapper for Mapper1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            // PRG: bank mode and prg_bank select which 16 KiB bank(s) appear at $8000 and $C000.
            0x8000..=0xFFFF => {
                let bank_mode = self.prg_bank_mode();
                let bank_count = self.prg_bank_count();

                let addr = addr as usize;

                match bank_mode {
                    0 | 1 => {
                        let bank = (self.prg_bank & !1) as usize;
                        let offset = addr - 0x8000;
                        self.prg_rom[(bank * 0x4000 * 2) + offset]
                    }
                    2 => {
                        if addr < 0xC000 {
                            self.prg_rom[addr - 0x8000]
                        } else {
                            let bank = self.prg_bank as usize;
                            self.prg_rom[(bank * 0x4000) + (addr - 0xC000)]
                        }
                    }
                    3 => {
                        if addr < 0xC000 {
                            let bank = self.prg_bank as usize;
                            self.prg_rom[(bank * 0x4000) + (addr - 0x8000)]
                        } else {
                            let bank = bank_count - 1;
                            self.prg_rom[(bank * 0x4000) + (addr - 0xC000)]
                        }
                    }
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        // MMC1: write with bit 7 set resets shift register (and often control to $0C).
        if data & 0x80 != 0 {
            self.shift_reg = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            return;
        }

        // Shift in LSB (bit 0); after 5 writes, latch to the register selected by address.
        self.shift_reg >>= 1;
        self.shift_reg |= (data & 1) << 4;
        self.shift_count += 1;

        if self.shift_count < 5 {
            return;
        }

        match addr {
            0x8000..=0x9FFF => self.control = self.shift_reg & 0x1F, // Control: mirroring, PRG/CHR mode
            0xE000..=0xFFFF => self.prg_bank = self.shift_reg & 0x0F, // PRG bank (4-bit)
            _ => {} // CHR0/CHR1 ($A000–$BFFF, $C000–$DFFF) not implemented
        }

        self.shift_reg = 0;
        self.shift_count = 0;
    }

    /// Mirroring from control bits 0–1: 0 = one-screen lower, 1 = one-screen upper, 2 = vertical, 3 = horizontal.
    fn mirroring(&mut self) -> Mirroring {
        match self.control & 0b11 {
            0 => Mirroring::OneScreenLower,
            1 => Mirroring::OneScreenUpper,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }
}
