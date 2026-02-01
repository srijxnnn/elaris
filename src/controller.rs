//! NES controller input handling.
//!
//! Implements the standard NES controller shift register protocol.

/// Represents a single NES controller connected to port 1.
pub struct Controller {
    pub state: u8,   // Current button states (A, B, Select, Start, Up, Down, Left, Right)
    pub shift: u8, // Shift register for serial output
}

impl Controller {
    /// Create a new controller with no buttons pressed.
    pub fn new() -> Self {
        Controller { state: 0, shift: 0 }
    }

    /// Read one button state; returns current bit OR'd with open bus ($40).
    pub fn read(&mut self) -> u8 {
        let bit = self.shift & 1;
        self.shift >>= 1;
        bit | 0x40
    }

    /// Latch: when bit 0 is 1, copy current button state into shift register.
    pub fn write(&mut self, data: u8) {
        if data & 1 != 0 {
            self.shift = self.state;
        }
    }
}
