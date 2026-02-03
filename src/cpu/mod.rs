//! 6502 CPU emulation for the NES (Ricoh 2A03).
//!
//! Implements the [CPU](https://www.nesdev.org/wiki/CPU) / [Instruction reference](https://www.nesdev.org/wiki/Instruction_reference):
//! all official 6502 opcodes plus [unofficial/undocumented opcodes](https://www.nesdev.org/wiki/CPU_unofficial_opcodes)
//! used by NES software. nestest-compatible. Bus trait abstracts [CPU memory map](https://www.nesdev.org/wiki/CPU_memory_map).
//! NMI from PPU vblank; reset vector from $FFFC–$FFFD. No DMC/APU IRQ cycle-accurate stall (handled in main loop).

pub mod cpu;
pub mod flags;
