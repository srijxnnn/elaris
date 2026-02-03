//! NES APU (Audio Processing Unit) emulation.
//!
//! - **Pulse** (×2): square waves with duty, envelope, sweep, length counter.
//! - **Triangle**: 32-step wave, linear counter, length counter.
//! - **Noise**: LFSR-based, envelope, length counter.
//! - **Frame counter**: 4-step or 5-step mode; clocks envelope/linear/length/sweep.
//! - **Mixer**: NES-style non-linear mix; output sampled at 44.1 kHz.
//!
//! DMC (sample playback) is not implemented.

pub mod apu;
