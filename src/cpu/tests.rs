use crate::{
    bus::Bus,
    cpu::{
        cpu::CPU,
        flags::{FLAG_NEGATIVE, FLAG_ZERO},
    },
};

struct TestBus {
    mem: [u8; 65536],
}

impl TestBus {
    fn new() -> Self {
        Self { mem: [0; 65536] }
    }
}

impl Bus for TestBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.mem[addr as usize] = data;
    }
}

fn new_cpu(bus: TestBus) -> CPU<TestBus> {
    CPU {
        a: 0,
        x: 0,
        y: 0,
        sp: 0,
        pc: 0,
        status: 0,
        cycles: 0,
        bus,
        halted: false,
    }
}

#[test]
fn lda_immediate_loads_value() {
    let mut bus = TestBus::new();
    bus.mem[0x8000] = 0xA9; // LDA #$42
    bus.mem[0x8001] = 0x42;

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);

    cpu.reset();
    cpu.step();

    assert_eq!(cpu.a, 0x42)
}

#[test]
fn lda_sets_zero_flag() {
    let mut bus = TestBus::new();
    bus.mem[0x8000] = 0xA9; // LDA #$00
    bus.mem[0x8001] = 0x00;

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);

    cpu.reset();
    cpu.step();
    assert!(cpu.status & FLAG_ZERO != 0)
}

#[test]
fn lda_sets_negative_flag() {
    let mut bus = TestBus::new();
    bus.mem[0x8000] = 0xA9; // LDA #$80
    bus.mem[0x8001] = 0x80;

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);

    cpu.reset();
    cpu.step(); // LDA

    assert!(cpu.status & FLAG_NEGATIVE != 0)
}

#[test]
fn tax_transfers_a_to_x() {
    let mut bus = TestBus::new();
    bus.mem[0x8000] = 0xA9; // LDA #$10
    bus.mem[0x8001] = 0x10;

    bus.mem[0x8002] = 0xAA; // TAX

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    cpu.step(); // LDA
    cpu.step(); // TAX

    assert_eq!(cpu.x, 0x10)
}

#[test]
fn sta_writes_to_memory() {
    let mut bus = TestBus::new();

    bus.mem[0x8000] = 0xA9; // LDA #$33
    bus.mem[0x8001] = 0x33;

    bus.mem[0x8002] = 0x8D; // STA $0200
    bus.mem[0x8003] = 0x00;
    bus.mem[0x8004] = 0x02;

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    cpu.step(); // LDA
    cpu.step(); // STA

    assert_eq!(cpu.bus.mem[0x0200], 0x33);
}

#[test]
fn jmp_changes_program_counter() {
    let mut bus = TestBus::new();

    bus.mem[0x8000] = 0x4C; // JMP $9000
    bus.mem[0x8001] = 0x00;
    bus.mem[0x8002] = 0x90;

    bus.mem[0x9000] = 0xA9; // LDA #$55
    bus.mem[0x9001] = 0x55;

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    cpu.step(); // JMP
    cpu.step(); // LDA

    assert_eq!(cpu.a, 0x55);
}

#[test]
fn inx_increments_x() {
    let mut bus = TestBus::new();

    bus.mem[0x8000] = 0xA2; // LDX #$01
    bus.mem[0x8001] = 0x01;
    bus.mem[0x8002] = 0xE8; // INX

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    cpu.step(); // LDX
    cpu.step(); // INX

    assert_eq!(cpu.x, 0x02);
}

#[test]
fn dex_sets_zero_flag() {
    let mut bus = TestBus::new();

    bus.mem[0x8000] = 0xA2; // LDX #$01
    bus.mem[0x8001] = 0x01;
    bus.mem[0x8002] = 0xCA; // DEX

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    cpu.step(); // LDX
    cpu.step(); // DEX

    assert!(cpu.status & FLAG_ZERO != 0);
}

#[test]
fn bne_loops_until_zero() {
    let mut bus = TestBus::new();

    bus.mem[0x8000] = 0xA2; // LDX #3
    bus.mem[0x8001] = 0x03;
    bus.mem[0x8002] = 0xCA; // DEX
    bus.mem[0x8003] = 0xD0; // BNE -3
    bus.mem[0x8004] = 0xFD; // -3 offset

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    for _ in 0..6 {
        cpu.step();
    }

    assert_eq!(cpu.x, 0x00);
}

#[test]
fn jsr_and_rts_work() {
    let mut bus = TestBus::new();

    // main program
    bus.mem[0x8000] = 0x20; // JSR $9000
    bus.mem[0x8001] = 0x00;
    bus.mem[0x8002] = 0x90;
    bus.mem[0x8003] = 0xA9; // LDA #$11
    bus.mem[0x8004] = 0x11;

    // subroutine
    bus.mem[0x9000] = 0xA9; // LDA #$22
    bus.mem[0x9001] = 0x22;
    bus.mem[0x9002] = 0x60; // RTS

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;

    let mut cpu = new_cpu(bus);
    cpu.reset();

    cpu.step(); // JSR
    cpu.step(); // LDA #$22
    cpu.step(); // RTS
    cpu.step(); // LDA #$11

    assert_eq!(cpu.a, 0x11);
}

#[test]
fn brk_jumps_to_irq_vector() {
    let mut bus = TestBus::new();

    bus.mem[0x8000] = 0x00; // BRK

    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0x80;
    bus.mem[0xFFFE] = 0x00;
    bus.mem[0xFFFF] = 0x90;

    let mut cpu = new_cpu(bus);
    cpu.reset();
    cpu.step();

    assert_eq!(cpu.pc, 0x9000);
}
