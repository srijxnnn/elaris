//! NES APU (Audio Processing Unit) emulation.
//!
//! Implements the [APU](https://www.nesdev.org/wiki/APU): pulse×2, triangle, noise, DMC, [frame
//! counter](https://www.nesdev.org/wiki/APU_Frame_Counter), and [mixer](https://www.nesdev.org/wiki/APU_Mixer).
//! Registers $4000–$4013, $4015, $4017. Output resampled to 44.1 kHz.

pub mod apu;
