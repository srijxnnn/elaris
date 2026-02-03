//! NES controller input handling.
//!
//! Implements the standard NES controller shift register protocol:
//! write $01 to $4016 to latch current state; then read $4016 repeatedly
//! to get one bit per read (A, B, Select, Start, Up, Down, Left, Right).

/// Represents a single NES controller connected to port 1 ($4016).
pub struct Controller {
    /// Current button states: bit 0 = A, 1 = B, 2 = Select, 3 = Start, 4 = Up, 5 = Down, 6 = Left, 7 = Right.
    pub state: u8,
    /// Shift register: latched from `state` on write; shifted out LSB-first on read.
    pub shift: u8,
}

impl Controller {
    /// Create a new controller with no buttons pressed.
    pub fn new() -> Self {
        Controller { state: 0, shift: 0 }
    }

    /// Read one button state from $4016. Returns LSB of shift register OR'd with open bus ($40).
    /// Each read advances the shift (next read = next button).
    pub fn read(&mut self) -> u8 {
        let bit = self.shift & 1;
        self.shift >>= 1;
        bit | 0x40
    }

    /// Write to $4016. When bit 0 is 1, latch current button state into the shift register.
    pub fn write(&mut self, data: u8) {
        if data & 1 != 0 {
            self.shift = self.state;
        }
    }
}
