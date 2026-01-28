use elaris::{bus::NesBus, cartridge::Cartridge, cpu::cpu::CPU};

fn main() {
    let cart = Cartridge::load("test/nestest.nes");
    let bus = NesBus::new(cart);

    let mut cpu = CPU {
        a: 0,
        x: 0,
        y: 0,
        sp: 0xFD,
        pc: 0,
        status: 0x24,
        cycles: 7,
        bus,
        halted: false,
    };

    // nestest requirement
    cpu.pc = 0xC000;

    loop {
        cpu.step();
    }
}
