//! 6502 processor status register (P) flag bits.
//!
//! See [Status flags](https://www.nesdev.org/wiki/Status_flags) and
//! [CPU](https://www.nesdev.org/wiki/CPU). The NES uses the Ricoh 2A03, which omits the 6502's
//! decimal mode (D flag is stored but not used; ADC/SBC behave as if D=0).

pub const FLAG_CARRY: u8 = 1 << 0;       // C: carry from ALU
pub const FLAG_ZERO: u8 = 1 << 1;       // Z: result zero
pub const FLAG_INTERRUPT_DISABLE: u8 = 1 << 2;  // I: maskable IRQ disabled (set by SEI, clear by CLI)
pub const FLAG_DECIMAL: u8 = 1 << 3;    // D: decimal mode; 2A03 does not use it (always 0 effective)
pub const FLAG_BREAK: u8 = 1 << 4;      // B: 1 in stack frame for BRK/IRQ (not a real register bit)
pub const FLAG_UNUSED: u8 = 1 << 5;     // Always 1 when P is read on 6502/2A03
pub const FLAG_OVERFLOW: u8 = 1 << 6;   // V: signed overflow (e.g. ADC, SBC, BIT)
pub const FLAG_NEGATIVE: u8 = 1 << 7;   // N: result bit 7 (sign)
