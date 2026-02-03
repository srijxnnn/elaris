//! Mapper 4 (MMC3): bank switching, switchable mirroring, optional PRG RAM, scanline IRQ.
//!
//! [MMC3](https://www.nesdev.org/wiki/MMC3): Bank select at $8000–$9FFE (even), bank data at
//! $8001–$9FFF (odd). R0/R1 = 2 KiB CHR, R2–R5 = 1 KiB CHR, R6/R7 = 8 KiB PRG. Mirroring at
//! $A000–$BFFE (even). IRQ latch $C000, reload $C001, disable $E000, enable $E001. IRQ counter
//! clocks on PPU CHR A12 rising edge (tracked in read when addr is in CHR range).

use crate::cartridge::mapper::{Mirroring, mapper::Mapper};

/// MMC3 state: bank registers, mirroring, PRG RAM, IRQ counter/latch/enable.
pub struct Mapper4 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    /// Bank select ($8000): bits 0–2 = register index, bit 6 = PRG mode, bit 7 = CHR A12 invert.
    bank_select: u8,
    /// R0–R5 CHR, R6–R7 PRG (R0/R1 are 2 KiB, others 1 KiB / 8 KiB).
    regs: [u8; 8],
    mirroring: Mirroring,
    /// PRG RAM enable (bit 7 of $A001). We leave RAM always enabled for MMC6 compatibility.
    _prg_ram_enable: bool,
    /// PRG RAM write protect (bit 6 of $A001). Not enforced for compatibility.
    _prg_ram_write_protect: bool,
    /// IRQ latch ($C000), counter, reload flag, enabled ($E001).
    irq_latch: u8,
    irq_counter: u8,
    irq_reload_pending: bool,
    irq_enabled: bool,
    irq_pending: bool,
    /// Previous PPU A12 (from CHR address) to detect rising edge.
    last_chr_a12: bool,
}

impl Mapper4 {
    /// Create MMC3 with PRG ROM and CHR ROM. PRG RAM 8 KiB is allocated for save RAM.
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Self {
            prg_rom,
            chr_rom,
            prg_ram: vec![0; 8 * 1024],
            bank_select: 0,
            regs: [0; 8],
            mirroring: Mirroring::Vertical,
            _prg_ram_enable: true,
            _prg_ram_write_protect: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload_pending: false,
            irq_enabled: false,
            irq_pending: false,
            last_chr_a12: false,
        }
    }

    fn prg_bank_count(&self) -> usize {
        self.prg_rom.len() / 0x2000
    }

    /// Number of 1 KiB CHR banks.
    fn chr_bank_count_1k(&self) -> usize {
        self.chr_rom.len() / 0x400
    }

    /// Number of 2 KiB CHR banks (for R0/R1).
    fn chr_bank_count_2k(&self) -> usize {
        self.chr_rom.len() / 0x800
    }

    /// Clock IRQ counter on PPU A12 rising edge (call when CHR read address has A12 going 0→1).
    fn clock_irq(&mut self) {
        if self.irq_counter == 0 || self.irq_reload_pending {
            self.irq_counter = self.irq_latch;
            self.irq_reload_pending = false;
        } else {
            self.irq_counter -= 1;
        }
        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }

    /// CHR bank for 2 KiB slot (R0 or R1): register value with low bit cleared.
    fn chr_2k_bank(&self, reg: u8) -> usize {
        ((self.regs[reg as usize] & 0xFE) as usize) % self.chr_bank_count_2k().max(1)
    }

    /// CHR bank for 1 KiB slot (R2–R5).
    fn chr_1k_bank(&self, reg: u8) -> usize {
        (self.regs[reg as usize] as usize) % self.chr_bank_count_1k().max(1)
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let a = addr as usize;
        let chr_invert = (self.bank_select & 0x80) != 0;
        let n_1k = self.chr_bank_count_1k();
        if n_1k == 0 {
            return 0;
        }

        if !chr_invert {
            // Two 2 KiB at $0000–$0FFF, four 1 KiB at $1000–$1FFF
            match addr {
                0x0000..=0x07FF => {
                    let bank = self.chr_2k_bank(0);
                    self.chr_rom[bank * 0x800 + (a & 0x7FF)]
                }
                0x0800..=0x0FFF => {
                    let bank = self.chr_2k_bank(1);
                    self.chr_rom[bank * 0x800 + (a & 0x7FF)]
                }
                0x1000..=0x13FF => {
                    let bank = self.chr_1k_bank(2);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x1400..=0x17FF => {
                    let bank = self.chr_1k_bank(3);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x1800..=0x1BFF => {
                    let bank = self.chr_1k_bank(4);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x1C00..=0x1FFF => {
                    let bank = self.chr_1k_bank(5);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                _ => 0,
            }
        } else {
            // Four 1 KiB at $0000–$0FFF, two 2 KiB at $1000–$1FFF
            match addr {
                0x0000..=0x03FF => {
                    let bank = self.chr_1k_bank(2);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x0400..=0x07FF => {
                    let bank = self.chr_1k_bank(3);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x0800..=0x0BFF => {
                    let bank = self.chr_1k_bank(4);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x0C00..=0x0FFF => {
                    let bank = self.chr_1k_bank(5);
                    self.chr_rom[bank * 0x400 + (a & 0x3FF)]
                }
                0x1000..=0x17FF => {
                    let bank = self.chr_2k_bank(0);
                    self.chr_rom[bank * 0x800 + (a - 0x1000)]
                }
                0x1800..=0x1FFF => {
                    let bank = self.chr_2k_bank(1);
                    self.chr_rom[bank * 0x800 + (a - 0x1800)]
                }
                _ => 0,
            }
        }
    }
}

impl Mapper for Mapper4 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.read_chr(addr),
            0x6000..=0x7FFF => {
                let i = (addr - 0x6000) as usize;
                *self.prg_ram.get(i).unwrap_or(&0)
            }
            0x8000..=0xFFFF => {
                let prg_mode = (self.bank_select & 0x40) != 0;
                let bank_count = self.prg_bank_count().max(1);
                let last = bank_count - 1;
                let second_last = last.saturating_sub(1);
                let addr = addr as usize;

                let (bank_8000, bank_a000, bank_c000, bank_e000) = if !prg_mode {
                    let r6 = (self.regs[6] & 0x3F) as usize % bank_count;
                    let r7 = (self.regs[7] & 0x3F) as usize % bank_count;
                    (r6, r7, second_last, last)
                } else {
                    let r6 = (self.regs[6] & 0x3F) as usize % bank_count;
                    let r7 = (self.regs[7] & 0x3F) as usize % bank_count;
                    (second_last, r7, r6, last)
                };

                let segment = ((addr as usize) - 0x8000) >> 13;
                let offset_8k = (addr as usize) & 0x1FFF;
                let bank = match segment {
                    0 => bank_8000,
                    1 => bank_a000,
                    2 => bank_c000,
                    3 => bank_e000,
                    _ => last,
                };
                let phys = bank * 0x2000 + offset_8k;
                *self.prg_rom.get(phys).unwrap_or(&0)
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // CHR is ROM on standard MMC3; writes ignored (no CHR RAM on TxROM).
            }
            0x6000..=0x7FFF => {
                let i = (addr - 0x6000) as usize;
                if let Some(b) = self.prg_ram.get_mut(i) {
                    *b = data;
                }
            }
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    self.bank_select = data;
                } else {
                    let r = (self.bank_select & 7) as usize;
                    self.regs[r] = data;
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    self.mirroring = if data & 1 != 0 {
                        Mirroring::Vertical
                    } else {
                        Mirroring::Horizontal
                    };
                } else {
                    self._prg_ram_enable = data & 0x80 != 0;
                    self._prg_ram_write_protect = data & 0x40 != 0;
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    self.irq_latch = data;
                } else {
                    self.irq_reload_pending = true;
                    self.irq_counter = 0;
                }
            }
            0xE000..=0xFFFF => {
                if addr & 1 == 0 {
                    self.irq_enabled = false;
                    self.irq_pending = false;
                } else {
                    self.irq_enabled = true;
                }
            }
            _ => {}
        }
    }

    fn mirroring(&mut self) -> Mirroring {
        self.mirroring
    }

    /// Called when the PPU reads a CHR address; clocks IRQ counter on A12 rising edge.
    fn on_chr_access(&mut self, addr: u16) {
        let a12 = (addr & 0x1000) != 0;
        if !self.last_chr_a12 && a12 {
            self.clock_irq();
        }
        self.last_chr_a12 = a12;
    }

    fn poll_irq(&mut self) -> bool {
        let p = self.irq_pending;
        self.irq_pending = false;
        p
    }
}
