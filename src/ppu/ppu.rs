pub struct PPU {
    pub cycle: u16,
    pub scanline: i16,
    pub nmi: bool,
    pub vblank: bool,
    pub ctrl: u8,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            cycle: 0,
            scanline: -1,
            nmi: false,
            vblank: false,
            ctrl: 0,
        }
    }
    pub fn tick(&mut self) {
        self.cycle += 1;

        if self.scanline == 241 && self.cycle == 1 {
            self.vblank = true;
            if self.ctrl & 0x80 != 0 {
                self.nmi = true;
            }
        }

        if self.scanline == -1 && self.cycle == 1 {
            self.vblank = false;
        }

        if self.cycle == 341 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline == 261 {
                self.scanline = -1;
            }
        }
    }
}
