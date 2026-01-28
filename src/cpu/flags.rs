// Processor status register (P)
//
// Note: Bits 4 (B) and 5 (unused) do not exist as physical latches in the CPU.
//
// Bit layout (7 → 0):
//
//     7 6 5 4 3 2 1 0
//     N V 1 B D I Z C
//     | | | | | | | +-- Carry
//     | | | | | | +---- Zero
//     | | | | | +------ Interrupt Disable
//     | | | | +-------- Decimal Mode (unused on NES)
//     | | | +---------- Break (only set when pushed to stack)
//     | | +------------ Unused (always 1 when pushed)
//     | +-------------- Overflow
//     +---------------- Negative
//

// Reference: https://www.nesdev.org/wiki/Status_flags

pub const FLAG_CARRY: u8 = 1 << 0;
pub const FLAG_ZERO: u8 = 1 << 1;
pub const FLAG_INTERRUPT_DISABLE: u8 = 1 << 2;
pub const FLAG_DECIMAL: u8 = 1 << 3;
pub const FLAG_BREAK: u8 = 1 << 4;
pub const FLAG_UNUSED: u8 = 1 << 5;
pub const FLAG_OVERFLOW: u8 = 1 << 6;
pub const FLAG_NEGATIVE: u8 = 1 << 7;
