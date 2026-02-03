//! Mapper 1 (MMC1): bank switching via 5-bit shift register.
//!
//! Writes to $8000–$9FFF (control), $A000–$BFFF (CHR0), $C000–$DFFF (CHR1), $E000–$FFFF (PRG).
//! Each write shifts in one bit; after 5 writes the value is latched.

use crate::cartridge::mapper::{Mirroring, mapper::Mapper};

/// MMC1: shift register, control (mirroring, PRG/CHR mode), PRG bank. CHR banking omitted here.
pub struct Mapper1 {
    prg_rom: Vec<u8>,
    shift_reg: u8,
    shift_count: u8,
    control: u8,
    prg_bank: u8,
}

impl Mapper1 {
    /// Create Mapper1 with PRG ROM (CHR may be RAM or ROM depending on cartridge).
    pub fn new(prg_rom: Vec<u8>) -> Self {
        Self {
            prg_rom,
            shift_reg: 0,
            shift_count: 0,
            control: 0x0C,
            prg_bank: 0,
        }
    }

    /// Control register bits 2-3: PRG bank mode (0/1/2/3).
    fn prg_bank_mode(&self) -> u8 {
        (self.control >> 2) & 0b11
    }

    /// Number of 16KB PRG banks.
    fn prg_bank_count(&self) -> usize {
        self.prg_rom.len() / 0x4000
    }
}

impl Mapper for Mapper1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            // PRG ROM: bank mode determines $8000/$C000 mapping
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
        // Reset shift register when bit 7 set
        if data & 0x80 != 0 {
            self.shift_reg = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            return;
        }

        // Shift in bit 0; after 5 writes, latch and apply
        self.shift_reg >>= 1;
        self.shift_reg |= (data & 1) << 4;
        self.shift_count += 1;

        if self.shift_count < 5 {
            return;
        }

        match addr {
            // $8000-$9FFF: control register (mirroring, PRG/CHR mode)
            0x8000..=0x9FFF => {
                self.control = self.shift_reg & 0x1F;
            }
            // $E000-$FFFF: PRG bank select
            0xE000..=0xFFFF => {
                self.prg_bank = self.shift_reg & 0x0F;
            }
            _ => {}
        }

        self.shift_reg = 0;
        self.shift_count = 0;
    }

    fn mirroring(&mut self) -> Mirroring {
        // Control bits 0-1: mirroring (0/1=H, 2=V, 3=1-screen)
        match self.control & 0b11 {
            0 | 1 => Mirroring::Horizontal,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal, // one-screen ignored for now
            _ => unreachable!(),
        }
    }
}
