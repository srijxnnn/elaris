pub struct PPU {
    pub cycle: u16,
    pub scanline: i16,
    pub nmi: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            cycle: 0,
            scanline: -1,
            nmi: false,
        }
    }
    pub fn tick(&mut self) {
        self.cycle += 1;

        if self.scanline == 241 && self.cycle == 1 {
            self.nmi = true;
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
