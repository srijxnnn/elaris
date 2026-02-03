//! NES controller input handling.
//!
//! Implements the [Controller reading](https://www.nesdev.org/wiki/Controller_reading) protocol
//! for the [standard controller](https://www.nesdev.org/wiki/Standard_controller) on port 1 ($4016):
//!
//! 1. **Strobe (latch)**: Write 1 to $4016 to poll buttons; the 4021 shift register loads parallel
//!    inputs. Write 0 to return to serial mode.
//! 2. **Read**: Each read from $4016 returns one bit (LSB first) and advances the shift register.
//!    Order: A, B, Select, Start, Up, Down, Left, Right. Unused bits read as open bus (we use $40).
//!
//! Only the low 3 bits of the write are latched (controller port latch + expansion); we use bit 0
//! for strobe. D0 is the data line for the standard controller; $4017 (port 2) is not implemented.

/// Represents a single NES controller on port 1 ($4016).
/// Hardware: 4021 8-bit parallel-in/serial-out shift register; CLK on read, parallel load on strobe.
pub struct Controller {
    /// Current button states. Bit order: 0=A, 1=B, 2=Select, 3=Start, 4=Up, 5=Down, 6=Left, 7=Right.
    /// 1 = pressed. Latched into `shift` when game writes 1 to $4016 (strobe).
    pub state: u8,
    /// Shift register: after strobe, holds the latched state; each read outputs LSB and shifts right.
    pub shift: u8,
}

impl Controller {
    /// Create a new controller with no buttons pressed.
    pub fn new() -> Self {
        Controller { state: 0, shift: 0 }
    }

    /// Read $4016: returns one bit (D0) and advances the shift register. Upper bits are open bus ($40).
    /// So first read = A, second = B, … eighth = Right. After 8 reads, further reads typically
    /// return 1 (open bus or floating). See "Clock timing" on NESdev Controller_reading.
    pub fn read(&mut self) -> u8 {
        let bit = self.shift & 1;
        self.shift >>= 1;
        bit | 0x40
    }

    /// Write $4016. Bit 0 (controller latch): 1 = load shift register from current button state;
    /// 0 = no change. Game usually does: write 1, write 0, then 8 reads. See Controller_reading.
    pub fn write(&mut self, data: u8) {
        if data & 1 != 0 {
            self.shift = self.state;
        }
    }
}
