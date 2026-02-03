//! 6502 processor status register (P) flag bits.

pub const FLAG_CARRY: u8 = 1 << 0;
pub const FLAG_ZERO: u8 = 1 << 1;
pub const FLAG_INTERRUPT_DISABLE: u8 = 1 << 2;
pub const FLAG_DECIMAL: u8 = 1 << 3;  // NES APU ignores; always 0 effective
pub const FLAG_BREAK: u8 = 1 << 4;    // Set by BRK / IRQ stack frame
pub const FLAG_UNUSED: u8 = 1 << 5;   // Always 1 when read on 6502
pub const FLAG_OVERFLOW: u8 = 1 << 6;
pub const FLAG_NEGATIVE: u8 = 1 << 7;
