use core::panic;

use crate::{
    bus::Bus,
    cpu::flags::{
        FLAG_BREAK, FLAG_CARRY, FLAG_DECIMAL, FLAG_INTERRUPT_DISABLE, FLAG_NEGATIVE, FLAG_OVERFLOW,
        FLAG_UNUSED, FLAG_ZERO,
    },
};

use ansi_term::Colour::{Green, Red};

pub struct CPU<B: Bus> {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
    pub cycles: usize,
    pub bus: B,
    pub halted: bool,
}

impl<B: Bus> CPU<B> {
    pub fn reset(&mut self) {
        let lo = self.bus.read(0xFFFC) as u16;
        let hi = self.bus.read(0xFFFD) as u16;

        self.pc = (hi << 8) | lo;

        self.sp = 0xFD; // resets at 0xFD instead of 0xFF for some reason
        self.status = FLAG_INTERRUPT_DISABLE | FLAG_UNUSED;

        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.halted = false;

        self.cycles = 7;
    }

    pub fn step(&mut self) {
        if self.halted {
            return;
        }

        if self.bus.poll_nmi() {
            self.nmi();
        }

        let pc = self.pc;
        let opcode = self.fetch_byte();
        // println!(
        //     "{} executing opcode: ${:02X}",
        //     Green.bold().paint("INFO"),
        //     opcode
        // );
        self.trace(pc, opcode);
        let prev_cycles = self.cycles;
        self.execute_opcode(opcode);
        let cycle_diff = self.cycles - prev_cycles;
        self.bus.tick(cycle_diff);
    }

    fn jam(&mut self) {
        self.halted = true;
    }

    fn fetch_byte(&mut self) -> u8 {
        let byte = self.bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    fn fetch_word(&mut self) -> u16 {
        let lo = self.fetch_byte() as u16;
        let hi = self.fetch_byte() as u16;
        (hi << 8) | lo
    }

    fn trace(&self, pc: u16, opcode: u8) {
        println!(
            "{:04X}  {:02X}        A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            pc, opcode, self.a, self.x, self.y, self.status, self.sp, self.cycles
        );
    }

    fn execute_opcode(&mut self, opcode: u8) {
        match opcode {
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                self.jam()
            }
            0xEA => self.nop(),
            0x04 | 0x44 | 0x64 => self.nop_zeropage(),
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => self.nop_zeropage_x(),
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => self.nop_implied(),
            0x0C => self.nop_absolute(),
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => self.nop_absolute_x(),
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => self.nop_immediate(),
            0xA9 => self.lda_immediate(),
            0xA5 => self.lda_zero_page(),
            0xB5 => self.lda_zeropage_x(),
            0xAD => self.lda_absolute(),
            0xBD => self.lda_absolute_x(),
            0xB9 => self.lda_absolute_y(),
            0xA1 => self.lda_indirect_x(),
            0xB1 => self.lda_indirect_y(),
            0xA2 => self.ldx_immediate(),
            0xAE => self.ldx_absolute(),
            0xBE => self.ldx_absolute_y(),
            0xA6 => self.ldx_zeropage(),
            0xB6 => self.ldx_zeropage_y(),
            0xA0 => self.ldy_immediate(),
            0xAC => self.ldy_absolute(),
            0xBC => self.ldy_absolute_x(),
            0xA4 => self.ldy_zeropage(),
            0xB4 => self.ldy_zeropage_x(),
            0xAF => self.lax_absolute(),
            0xBF => self.lax_absolute_y(),
            0xA7 => self.lax_zeropage(),
            0xB7 => self.lax_zeropage_y(),
            0xA3 => self.lax_indirect_x(),
            0xB3 => self.lax_indirect_y(),
            0x85 => self.sta_zero_page(),
            0x95 => self.sta_zeropage_x(),
            0x8D => self.sta_absolute(),
            0x9D => self.sta_absolute_x(),
            0x99 => self.sta_absolute_y(),
            0x81 => self.sta_indirect_x(),
            0x91 => self.sta_indirect_y(),
            0x86 => self.stx_zero_page(),
            0x96 => self.stx_zeropage_y(),
            0x8E => self.stx_absolute(),
            0x84 => self.sty_zeropage(),
            0x94 => self.sty_zeropage_x(),
            0x8C => self.sty_absolute(),
            0x8F => self.sax_absolute(),
            0x87 => self.sax_zeropage(),
            0x97 => self.sax_zeropage_y(),
            0x83 => self.sax_indirect_x(),
            0x4C => self.jmp_absolute(),
            0x6C => self.jmp_indirect(),
            0x29 => self.and_immediate(),
            0x25 => self.and_zeropage(),
            0x35 => self.and_zeropage_x(),
            0x2D => self.and_absolute(),
            0x3D => self.and_absolute_x(),
            0x39 => self.and_absolute_y(),
            0x21 => self.and_indirect_x(),
            0x31 => self.and_indirect_y(),
            0x09 => self.ora_immediate(),
            0x0D => self.ora_absolute(),
            0x1D => self.ora_absolute_x(),
            0x19 => self.ora_absolute_y(),
            0x01 => self.ora_indirect_x(),
            0x11 => self.ora_indirect_y(),
            0x05 => self.ora_zeropage(),
            0x15 => self.ora_zeropage_x(),
            0x49 => self.eor_immediate(),
            0x45 => self.eor_zeropage(),
            0x55 => self.eor_zeropage_x(),
            0x4D => self.eor_absolute(),
            0x5D => self.eor_absolute_x(),
            0x59 => self.eor_absolute_y(),
            0x41 => self.eor_indirect_x(),
            0x51 => self.eor_indirect_y(),
            0x69 => self.adc_immediate(),
            0x6D => self.adc_absolute(),
            0x7D => self.adc_absolute_x(),
            0x79 => self.adc_absolute_y(),
            0x65 => self.adc_zeropage(),
            0x75 => self.adc_zeropage_x(),
            0x61 => self.adc_indirect_x(),
            0x71 => self.adc_indirect_y(),
            0xEB | 0xE9 => self.sbc_immediate(),
            0xED => self.sbc_absolute(),
            0xFD => self.sbc_absolute_x(),
            0xF9 => self.sbc_absolute_y(),
            0xE5 => self.sbc_zeropage(),
            0xF5 => self.sbc_zeropage_x(),
            0xE1 => self.sbc_indirect_x(),
            0xF1 => self.sbc_indirect_y(),
            0xE8 => self.inx(),
            0xE6 => self.inc_zeropage(),
            0xF6 => self.inc_zeropage_x(),
            0xEE => self.inc_absolute(),
            0xFE => self.inc_absolute_x(),
            0xC8 => self.iny(),
            0xEF => self.isc_absolute(),
            0xFF => self.isc_absolute_x(),
            0xFB => self.isc_absolute_y(),
            0xE7 => self.isc_zeropage(),
            0xF7 => self.isc_zeropage_x(),
            0xE3 => self.isc_indirect_x(),
            0xF3 => self.isc_indirect_y(),
            0x88 => self.dey(),
            0xCA => self.dex(),
            0xCE => self.dec_absolute(),
            0xDE => self.dec_absolute_x(),
            0xC6 => self.dec_zeropage(),
            0xD6 => self.dec_zeropage_x(),
            0xC9 => self.cmp_immediate(),
            0xCD => self.cmp_absolute(),
            0xDD => self.cmp_absolute_x(),
            0xD9 => self.cmp_absolute_y(),
            0xC5 => self.cmp_zeropage(),
            0xD5 => self.cmp_zeropage_x(),
            0xC1 => self.cmp_indirect_x(),
            0xD1 => self.cmp_indirect_y(),
            0xCF => self.dcp_absolute(),
            0xDF => self.dcp_absolute_x(),
            0xDB => self.dcp_absolute_y(),
            0xC7 => self.dcp_zeropage(),
            0xD7 => self.dcp_zeropage_x(),
            0xC3 => self.dcp_indirect_x(),
            0xD3 => self.dcp_indirect_y(),
            0xC0 => self.cpy_immediate(),
            0xCC => self.cpy_absolute(),
            0xC4 => self.cpy_zeropage(),
            0xE0 => self.cpx_immediate(),
            0xEC => self.cpx_absolute(),
            0xE4 => self.cpx_zeropage(),
            0xF8 => self.sed(),
            0xF0 => self.beq(),
            0xD0 => self.bne(),
            0xB0 => self.bcs(),
            0x90 => self.bcc(),
            0x70 => self.bvs(),
            0x50 => self.bvc(),
            0x10 => self.bpl(),
            0x30 => self.bmi(),
            0x20 => self.jsr(),
            0x60 => self.rts(),
            0x40 => self.rti(),
            0x48 => self.pha(),
            0x08 => self.php(),
            0x28 => self.plp(),
            0x68 => self.pla(),
            0x00 => self.brk(),
            0x78 => self.sei(),
            0x58 => self.cli(),
            0x98 => self.tya(),
            0x8A => self.txa(),
            0xAA => self.tax(),
            0xA8 => self.tay(),
            0xBA => self.tsx(),
            0x9A => self.txs(),
            0x38 => self.sec(),
            0x18 => self.clc(),
            0xD8 => self.cld(),
            0xB8 => self.clv(),
            0x4A => self.lsr_accumulator(),
            0x46 => self.lsr_zeropage(),
            0x56 => self.lsr_zeropage_x(),
            0x4E => self.lsr_absolute(),
            0x5E => self.lsr_absolute_x(),
            0x4F => self.sre_absolute(),
            0x5F => self.sre_absolute_x(),
            0x5B => self.sre_absolute_y(),
            0x47 => self.sre_zeropage(),
            0x57 => self.sre_zeropage_x(),
            0x43 => self.sre_indirect_x(),
            0x53 => self.sre_indirect_y(),
            0x0A => self.asl_accumulator(),
            0x0E => self.asl_absolute(),
            0x1E => self.asl_absolute_x(),
            0x06 => self.asl_zeropage(),
            0x16 => self.asl_zeropage_x(),
            0x0F => self.slo_absolute(),
            0x1F => self.slo_absolute_x(),
            0x1B => self.slo_absolute_y(),
            0x07 => self.slo_zeropage(),
            0x17 => self.slo_zeropage_x(),
            0x03 => self.slo_indirect_x(),
            0x13 => self.slo_indirect_y(),
            0x2F => self.rla_absolute(),
            0x3F => self.rla_absolute_x(),
            0x3B => self.rla_absolute_y(),
            0x27 => self.rla_zeropage(),
            0x37 => self.rla_zeropage_x(),
            0x23 => self.rla_indirect_x(),
            0x33 => self.rla_indirect_y(),
            0x6A => self.ror_accumulator(),
            0x6E => self.ror_absolute(),
            0x7E => self.ror_absolute_x(),
            0x66 => self.ror_zeropage(),
            0x76 => self.ror_zeropage_x(),
            0x6F => self.rra_absolute(),
            0x7F => self.rra_absolute_x(),
            0x7B => self.rra_absolute_y(),
            0x67 => self.rra_zeropage(),
            0x77 => self.rra_zeropage_x(),
            0x63 => self.rra_indirect_x(),
            0x73 => self.rra_indirect_y(),
            0x2A => self.rol_accumulator(),
            0x2E => self.rol_absolute(),
            0x3E => self.rol_absolute_x(),
            0x26 => self.rol_zeropage(),
            0x36 => self.rol_zeropage_x(),
            0x24 => self.bit_zeropage(),
            0x2C => self.bit_absolute(),
            _ => panic!(
                "{} unimplemented opcode: ${:02X}",
                Red.bold().paint("ERROR"),
                opcode
            ),
        }
    }

    fn lda_immediate(&mut self) {
        let value = self.fetch_byte();
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn lda_zero_page(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 3;
    }

    fn lda_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 4;
    }

    fn lda_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 4;
    }

    fn lda_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let value = self.bus.read(final_addr);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn lda_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn lda_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;

        let addr = (hi << 8) | lo;
        let value = self.bus.read(addr);

        self.a = value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 6;
    }

    fn lda_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ldx_immediate(&mut self) {
        let value = self.fetch_byte();
        self.x = value;
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 2;
    }

    fn ldx_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);
        self.x = value;
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 4;
    }

    fn ldx_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.x = value;
        self.update_zero_and_negative_flags(self.x);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ldx_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        self.x = value;
        self.update_zero_and_negative_flags(self.x);

        self.cycles += 3;
    }

    fn ldx_zeropage_y(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.y) as u16;

        let value = self.bus.read(addr);
        self.x = value;
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 4;
    }

    fn ldy_immediate(&mut self) {
        let value = self.fetch_byte();
        self.y = value;
        self.update_zero_and_negative_flags(self.y);
        self.cycles += 2;
    }

    fn ldy_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        self.y = value;
        self.update_zero_and_negative_flags(self.y);

        self.cycles += 4;
    }

    fn ldy_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let value = self.bus.read(final_addr);
        self.y = value;
        self.update_zero_and_negative_flags(self.y);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ldy_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        self.y = value;
        self.update_zero_and_negative_flags(self.y);
        self.cycles += 3;
    }

    fn ldy_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);
        self.y = value;
        self.update_zero_and_negative_flags(self.y);
        self.cycles += 4;
    }

    fn lax_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        self.a = value;
        self.x = value;
        self.update_zero_and_negative_flags(value);

        self.cycles += 4;
    }

    fn lax_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);

        self.a = value;
        self.x = value;
        self.update_zero_and_negative_flags(value);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn lax_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        self.a = value;
        self.x = value;
        self.update_zero_and_negative_flags(value);

        self.cycles += 3;
    }

    fn lax_zeropage_y(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.y) as u16;

        let value = self.bus.read(addr);

        self.a = value;
        self.x = value;
        self.update_zero_and_negative_flags(value);

        self.cycles += 4;
    }

    fn lax_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);

        self.a = value;
        self.x = value;
        self.update_zero_and_negative_flags(value);

        self.cycles += 6;
    }

    fn lax_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        self.a = value;
        self.x = value;
        self.update_zero_and_negative_flags(value);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 2;
    }

    fn tay(&mut self) {
        self.y = self.a;
        self.update_zero_and_negative_flags(self.y);
        self.cycles += 2;
    }

    fn tsx(&mut self) {
        self.x = self.sp;
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 2;
    }

    fn txs(&mut self) {
        self.sp = self.x;
        self.cycles += 2;
    }

    fn sta_zero_page(&mut self) {
        let addr = self.fetch_byte() as u16;
        self.bus.write(addr, self.a);
        self.cycles += 3;
    }

    fn sta_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        self.bus.write(addr, self.a);
        self.cycles += 4;
    }

    fn sta_absolute(&mut self) {
        let addr = self.fetch_word();
        self.bus.write(addr, self.a);
        self.cycles += 4;
    }

    fn sta_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        self.bus.write(final_addr, self.a);
        self.cycles += 5;
    }

    fn sta_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        self.bus.write(final_addr, self.a);

        self.cycles += 5;
    }

    fn sta_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        self.bus.write(addr, self.a);

        self.cycles += 6;
    }

    fn sta_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        self.bus.write(final_addr, self.a);
        self.cycles += 6;
    }

    fn stx_zero_page(&mut self) {
        let addr = self.fetch_byte() as u16;
        self.bus.write(addr, self.x);
        self.cycles += 3;
    }

    fn stx_zeropage_y(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.y) as u16;

        self.bus.write(addr, self.x);
        self.cycles += 4;
    }

    fn stx_absolute(&mut self) {
        let addr = self.fetch_word();
        self.bus.write(addr, self.x);
        self.cycles += 4;
    }

    fn sty_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        self.bus.write(addr, self.y);
        self.cycles += 3;
    }

    fn sty_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        self.bus.write(addr, self.y);

        self.cycles += 4;
    }

    fn sty_absolute(&mut self) {
        let addr = self.fetch_word();
        self.bus.write(addr, self.y);
        self.cycles += 4;
    }

    fn sax_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.a & self.x;

        self.bus.write(addr, value);
        self.cycles += 4;
    }

    fn sax_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.a & self.x;

        self.bus.write(addr, value);
        self.cycles += 3;
    }

    fn sax_zeropage_y(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.y) as u16;

        let value = self.a & self.x;
        self.bus.write(addr, value);

        self.cycles += 4;
    }

    fn sax_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let value = self.a & self.x;
        self.bus.write(addr, value);
        self.cycles += 6;
    }

    fn and_immediate(&mut self) {
        let value = self.fetch_byte();
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn and_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 3;
    }

    fn and_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn and_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn and_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let value = self.bus.read(final_addr);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn and_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn and_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);
        self.a &= value;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn and_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ora_immediate(&mut self) {
        let value = self.fetch_byte();
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn ora_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn ora_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let value = self.bus.read(final_addr);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ora_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ora_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);
        self.a |= value;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn ora_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn ora_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 3;
    }

    fn ora_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn eor_immediate(&mut self) {
        let value = self.fetch_byte();
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn eor_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 3;
    }

    fn eor_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn eor_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn eor_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let value = self.bus.read(final_addr);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn eor_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn eor_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);
        self.a ^= value;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn eor_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        let value = self.bus.read(final_addr);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn adc_immediate(&mut self) {
        let value = self.fetch_byte();
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn adc_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn adc_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);
        let value = self.bus.read(final_addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn adc_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn adc_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 3;
    }

    fn adc_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn adc_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 6;
    }

    fn adc_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn sbc_immediate(&mut self) {
        let value = self.fetch_byte();
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if (((self.a ^ result) & (result ^ value)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;

        self.update_zero_and_negative_flags(self.a);

        self.cycles += 2;
    }

    fn sbc_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;
    }

    fn sbc_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);
        let value = self.bus.read(final_addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn sbc_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn sbc_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 3;
    }

    fn sbc_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 4;
    }

    fn sbc_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 6;
    }

    fn sbc_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        let value = value ^ 0xFF;
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ value) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn jmp_absolute(&mut self) {
        let addr = self.fetch_word();
        self.pc = addr;
        self.cycles += 3;
    }

    fn jmp_indirect(&mut self) {
        let addr = self.fetch_word();

        let lo = self.bus.read(addr) as u16;

        let hi_addr = (addr & 0xFF00) | ((addr + 1) & 0x00FF); // page-boundary bug
        let hi = self.bus.read(hi_addr) as u16;

        self.pc = (hi << 8) | lo;

        self.cycles += 5;
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 2;
    }

    fn inc_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);

        value = value.wrapping_add(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 5;
    }

    fn inc_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);
        value = value.wrapping_add(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn inc_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        value = value.wrapping_add(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn inc_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);
        value = value.wrapping_add(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 7;
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
        self.cycles += 2;
    }

    fn isc_absolute(&mut self) {
        let addr = self.fetch_word();

        let mut value = self.bus.read(addr);
        value = value.wrapping_add(1);
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn isc_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(final_addr);
        value = value.wrapping_add(1);
        self.bus.write(final_addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 7;
    }

    fn isc_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(final_addr);
        value = value.wrapping_add(1);
        self.bus.write(final_addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 7;
    }

    fn isc_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;

        let mut value = self.bus.read(addr);
        value = value.wrapping_add(1);
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;
    }

    fn isc_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);
        value = value.wrapping_add(1);
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn isc_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let mut value = self.bus.read(addr);
        value = value.wrapping_add(1);
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 8;
    }

    fn isc_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(final_addr);
        value = value.wrapping_add(1);
        self.bus.write(final_addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let inv = value ^ 0xFF;

        let sum = self.a as u16 + inv as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((self.a ^ result) & (result ^ inv) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 8;
    }

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.x);
        self.cycles += 2;
    }

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.y);
        self.cycles += 2;
    }

    fn dec_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        value = value.wrapping_sub(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn dec_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);
        value = value.wrapping_sub(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 7;
    }

    fn dec_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);

        value = value.wrapping_sub(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 5;
    }

    fn dec_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);
        value = value.wrapping_sub(1);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn beq(&mut self) {
        let zero = self.status & FLAG_ZERO != 0;
        self.branch(zero);
    }

    fn bne(&mut self) {
        let zero = self.status & FLAG_ZERO != 0;
        self.branch(!zero);
    }

    fn bcs(&mut self) {
        let carry_set = self.status & FLAG_CARRY != 0;
        self.branch(carry_set);
    }

    fn bcc(&mut self) {
        let carry_clear = (self.status & FLAG_CARRY) == 0;
        self.branch(carry_clear);
    }

    fn bvs(&mut self) {
        let overflow_set = (self.status & FLAG_OVERFLOW) != 0;
        self.branch(overflow_set);
    }

    fn bvc(&mut self) {
        let overflow_clear = (self.status & FLAG_OVERFLOW) == 0;
        self.branch(overflow_clear);
    }

    fn bpl(&mut self) {
        let negative_clear = (self.status & FLAG_NEGATIVE) == 0;
        self.branch(negative_clear);
    }

    fn bmi(&mut self) {
        let negative_set = (self.status & FLAG_NEGATIVE) != 0;
        self.branch(negative_set);
    }

    fn cmp_immediate(&mut self) {
        let value = self.fetch_byte();
        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 2;
    }

    fn cmp_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 4;
    }

    fn cmp_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);
        let value = self.bus.read(final_addr);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn cmp_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn cmp_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 3;
    }

    fn cmp_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let value = self.bus.read(addr);
        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 4;
    }

    fn cmp_indirect_x(&mut self) {
        let zp = self.fetch_byte();
        let ptr = zp.wrapping_add(self.x) as u16;

        let lo = self.bus.read(ptr & 0x00FF) as u16;
        let hi = self.bus.read((ptr.wrapping_add(1)) & 0x00FF) as u16;
        let addr = (hi << 8) | lo;

        let value = self.bus.read(addr);
        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 6;
    }

    fn cmp_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);
        let value = self.bus.read(final_addr);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 5;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn dcp_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        value = value.wrapping_sub(1);

        self.bus.write(addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 6;
    }

    fn dcp_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(final_addr);
        value = value.wrapping_sub(1);
        self.bus.write(final_addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 7;
    }

    fn dcp_absolute_y(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(final_addr);
        value = value.wrapping_sub(1);
        self.bus.write(final_addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 7;
    }

    fn dcp_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);
        value = value.wrapping_sub(1);

        self.bus.write(addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 5;
    }

    fn dcp_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);
        value = value.wrapping_sub(1);
        self.bus.write(addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 6;
    }

    fn dcp_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let mut value = self.bus.read(addr);

        value = value.wrapping_sub(1);

        self.bus.write(addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 8;
    }

    fn dcp_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let final_addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(final_addr);
        value = value.wrapping_sub(1);

        self.bus.write(final_addr, value);

        let result = self.a.wrapping_sub(value);

        if self.a >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 8;
    }

    fn cpy_immediate(&mut self) {
        let value = self.fetch_byte();
        let result = self.y.wrapping_sub(value);

        if self.y >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 2;
    }

    fn cpy_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        let result = self.y.wrapping_sub(value);

        if self.y >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 4;
    }

    fn cpy_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        let result = self.y.wrapping_sub(value);

        if self.y >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 3;
    }

    fn cpx_immediate(&mut self) {
        let value = self.fetch_byte();
        let result = self.x.wrapping_sub(value);

        if self.x >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);

        self.cycles += 2;
    }

    fn cpx_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        let result = self.x.wrapping_sub(value);

        if self.x >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 4;
    }

    fn cpx_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        let result = self.x.wrapping_sub(value);

        if self.x >= value {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.update_zero_and_negative_flags(result);
        self.cycles += 3;
    }

    fn jsr(&mut self) {
        let addr = self.fetch_word();

        let return_addr = self.pc.wrapping_sub(1);
        self.push((return_addr >> 8) as u8);
        self.push(return_addr as u8);

        self.pc = addr;
        self.cycles += 6;
    }

    fn rti(&mut self) {
        let status = self.pop();
        self.status = (status & !FLAG_BREAK) | FLAG_UNUSED;

        let lo = self.pop() as u16;
        let hi = self.pop() as u16;
        self.pc = (hi << 8) | lo;

        self.cycles += 6;
    }

    fn rts(&mut self) {
        let lo = self.pop() as u16;
        let hi = self.pop() as u16;

        self.pc = ((hi << 8) | lo).wrapping_add(1);
        self.cycles += 6;
    }

    fn pha(&mut self) {
        self.push(self.a);
        self.cycles += 3;
    }

    fn php(&mut self) {
        let status = self.status | FLAG_BREAK | FLAG_UNUSED;
        self.push(status);
        self.cycles += 3;
    }

    fn plp(&mut self) {
        let value = self.pop();
        self.status = (value & !FLAG_BREAK) | FLAG_UNUSED;
        self.cycles += 4;
    }

    fn pla(&mut self) {
        self.a = self.pop();
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 4;
    }

    fn nop(&mut self) {
        self.cycles += 2;
    }

    fn nop_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let _ = self.bus.read(addr);
        self.cycles += 3;
    }

    fn nop_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;
        let _ = self.bus.read(addr);
        self.cycles += 4;
    }

    fn nop_absolute(&mut self) {
        let addr = self.fetch_word();
        let _ = self.bus.read(addr);
        self.cycles += 4;
    }

    fn nop_absolute_x(&mut self) {
        let base = self.fetch_word();
        let final_addr = base.wrapping_add(self.x as u16);

        let _ = self.bus.read(final_addr);

        self.cycles += 4;

        if (base & 0xFF00) != (final_addr & 0xFF00) {
            self.cycles += 1;
        }
    }

    fn nop_immediate(&mut self) {
        let _ = self.fetch_byte();
        self.cycles += 2;
    }

    fn nop_implied(&mut self) {
        self.cycles += 2;
    }

    fn sed(&mut self) {
        self.status |= FLAG_DECIMAL;
        self.cycles += 2;
    }

    fn sec(&mut self) {
        self.status |= FLAG_CARRY;
        self.cycles += 2;
    }

    fn sei(&mut self) {
        self.status |= FLAG_INTERRUPT_DISABLE;
        self.cycles += 2;
    }

    fn cli(&mut self) {
        self.status &= !FLAG_INTERRUPT_DISABLE;
        self.cycles += 2;
    }

    fn clc(&mut self) {
        self.status &= !FLAG_CARRY;
        self.cycles += 2;
    }

    fn cld(&mut self) {
        self.status &= !FLAG_DECIMAL;
        self.cycles += 2;
    }

    fn clv(&mut self) {
        self.status &= !FLAG_OVERFLOW;
        self.cycles += 2;
    }

    fn lsr_accumulator(&mut self) {
        if self.a & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.a >>= 1;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn lsr_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value >>= 1;

        self.bus.write(addr, value);

        self.update_zero_and_negative_flags(value);
        self.cycles += 5;
    }

    fn lsr_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value >>= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn lsr_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value >>= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn lsr_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value >>= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 7;
    }

    fn sre_absolute(&mut self) {
        let addr = self.fetch_word();

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn sre_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 7;
    }

    fn sre_absolute_y(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 7;
    }

    fn sre_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 5;
    }

    fn sre_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn sre_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 8;
    }

    fn sre_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value >>= 1;
        self.bus.write(addr, value);

        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 8;
    }

    fn asl_accumulator(&mut self) {
        if self.a & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.a <<= 1;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn asl_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn asl_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 7;
    }

    fn asl_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 5;
    }

    fn asl_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn slo_absolute(&mut self) {
        let addr = self.fetch_word();

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;
        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn slo_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;
        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 7;
    }

    fn slo_absolute_y(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;
        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 7;
    }

    fn slo_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;

        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 5;
    }

    fn slo_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;
        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn slo_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let mut value = self.bus.read(addr);
        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;
        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 8;
    }

    fn slo_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value <<= 1;
        self.bus.write(addr, value);

        self.a |= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 8;
    }

    fn rla_absolute(&mut self) {
        let addr = self.fetch_word();

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn rla_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 7;
    }

    fn rla_absolute_y(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 7;
    }

    fn rla_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 5;
    }

    fn rla_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn rla_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 8;
    }

    fn rla_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | carry_in;
        self.bus.write(addr, value);

        self.a &= value;
        self.update_zero_and_negative_flags(self.a);

        self.cycles += 8;
    }

    fn ror_accumulator(&mut self) {
        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if self.a & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.a = (self.a >> 1) | (old_carry << 7);

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn ror_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value >> 1) | (old_carry << 7);
        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn ror_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value >> 1) | (old_carry << 7);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 7;
    }

    fn ror_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value >> 1) | (old_carry << 7);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 5;
    }

    fn ror_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value >> 1) | (old_carry << 7);

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn rra_absolute(&mut self) {
        let addr = self.fetch_word();

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn rra_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 7;
    }

    fn rra_absolute_y(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 7;
    }

    fn rra_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 5;
    }

    fn rra_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 6;
    }

    fn rra_indirect_x(&mut self) {
        let base = self.fetch_byte();
        let ptr = base.wrapping_add(self.x);

        let lo = self.bus.read(ptr as u16) as u16;
        let hi = self.bus.read(ptr.wrapping_add(1) as u16) as u16;
        let addr = (hi << 8) | lo;

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 8;
    }

    fn rra_indirect_y(&mut self) {
        let zp = self.fetch_byte();

        let lo = self.bus.read(zp as u16) as u16;
        let hi = self.bus.read(zp.wrapping_add(1) as u16) as u16;
        let base = (hi << 8) | lo;

        let addr = base.wrapping_add(self.y as u16);

        let mut value = self.bus.read(addr);
        let carry_in = if self.status & FLAG_CARRY != 0 {
            0x80
        } else {
            0
        };

        if value & 0x01 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }
        value = (value >> 1) | carry_in;
        self.bus.write(addr, value);

        let carry_in = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };
        let sum = self.a as u16 + value as u16 + carry_in as u16;
        let result = sum as u8;

        if sum > 0xFF {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        if ((!(self.a ^ value) & (self.a ^ result)) & 0x80) != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.a = result;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 8;
    }

    fn rol_accumulator(&mut self) {
        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if self.a & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        self.a = (self.a << 1) | old_carry;

        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn rol_absolute(&mut self) {
        let addr = self.fetch_word();
        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | old_carry;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn rol_absolute_x(&mut self) {
        let base = self.fetch_word();
        let addr = base.wrapping_add(self.x as u16);

        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | old_carry;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 7;
    }

    fn rol_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | old_carry;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 5;
    }

    fn rol_zeropage_x(&mut self) {
        let base = self.fetch_byte();
        let addr = base.wrapping_add(self.x) as u16;

        let mut value = self.bus.read(addr);

        let old_carry = if self.status & FLAG_CARRY != 0 { 1 } else { 0 };

        if value & 0x80 != 0 {
            self.status |= FLAG_CARRY;
        } else {
            self.status &= !FLAG_CARRY;
        }

        value = (value << 1) | old_carry;

        self.bus.write(addr, value);
        self.update_zero_and_negative_flags(value);
        self.cycles += 6;
    }

    fn tya(&mut self) {
        self.a = self.y;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn txa(&mut self) {
        self.a = self.x;
        self.update_zero_and_negative_flags(self.a);
        self.cycles += 2;
    }

    fn brk(&mut self) {
        self.pc = self.pc.wrapping_add(1); // +1 because of padding byte

        self.push((self.pc >> 8) as u8);
        self.push(self.pc as u8);

        let status = self.status | FLAG_BREAK | FLAG_UNUSED;
        self.push(status);

        self.status |= FLAG_INTERRUPT_DISABLE;

        let lo = self.bus.read(0xFFFE) as u16;
        let hi = self.bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;

        self.cycles += 7;
    }

    fn bit_zeropage(&mut self) {
        let addr = self.fetch_byte() as u16;
        let value = self.bus.read(addr);

        if (self.a & value) == 0 {
            self.status |= FLAG_ZERO;
        } else {
            self.status &= !FLAG_ZERO;
        }

        if value & 0x80 != 0 {
            self.status |= FLAG_NEGATIVE;
        } else {
            self.status &= !FLAG_NEGATIVE;
        }

        if value & 0x40 != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.cycles += 3;
    }

    fn bit_absolute(&mut self) {
        let addr = self.fetch_word();
        let value = self.bus.read(addr);

        if (self.a & value) == 0 {
            self.status |= FLAG_ZERO;
        } else {
            self.status &= !FLAG_ZERO;
        }

        if value & 0x80 != 0 {
            self.status |= FLAG_NEGATIVE;
        } else {
            self.status &= !FLAG_NEGATIVE;
        }

        if value & 0x40 != 0 {
            self.status |= FLAG_OVERFLOW;
        } else {
            self.status &= !FLAG_OVERFLOW;
        }

        self.cycles += 4;
    }

    fn update_zero_and_negative_flags(&mut self, value: u8) {
        if value == 0 {
            self.status |= FLAG_ZERO;
        } else {
            self.status &= !FLAG_ZERO;
        }

        if value & 0x80 != 0 {
            self.status |= FLAG_NEGATIVE;
        } else {
            self.status &= !FLAG_NEGATIVE;
        }
    }

    fn irq(&mut self) {
        if self.status & FLAG_INTERRUPT_DISABLE != 0 {
            return;
        }

        self.push((self.pc >> 8) as u8);
        self.push(self.pc as u8);

        let status = (self.status & !FLAG_BREAK) | FLAG_UNUSED;
        self.push(status);

        self.status |= FLAG_INTERRUPT_DISABLE;

        let lo = self.bus.read(0xFFFE) as u16;
        let hi = self.bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;

        self.cycles += 7;
    }

    fn nmi(&mut self) {
        self.push((self.pc >> 8) as u8);
        self.push(self.pc as u8);

        let status = (self.status & !FLAG_BREAK) | FLAG_UNUSED;
        self.push(status);

        self.status |= FLAG_INTERRUPT_DISABLE;

        let lo = self.bus.read(0xFFFA) as u16;
        let hi = self.bus.read(0xFFFB) as u16;
        self.pc = (hi << 8) | lo;

        self.cycles += 7;
    }

    fn push(&mut self, value: u8) {
        let addr = 0x0100 | self.sp as u16;
        self.bus.write(addr, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 | self.sp as u16;
        self.bus.read(addr)
    }

    fn branch(&mut self, condition: bool) {
        let offset = self.fetch_byte() as i8;

        if condition {
            let old_pc = self.pc;
            self.pc = self.pc.wrapping_add(offset as u16);
            self.cycles += 1;

            if (old_pc & 0xFF00) != (self.pc & 0xFF00) {
                self.cycles += 1;
            }
        }

        self.cycles += 2;
    }
}
