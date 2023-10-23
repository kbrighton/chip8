pub static SP_OFFSET: u8 = 0x52;
pub static PROGRAM_OFFSET: u8=0x200;

struct Chip8 {
    registers: Registers,
    timers: Timers,
    memory: [u8; 4096]
}

struct Registers {
    index: u16,
    sp: u16,
    pc: u16,
    v: [u8; 16],
    rpl: [u8; 16]
}

struct Timers {
    delay: u8,
    sound: u8
}

impl Chip8 {
    fn reset(&mut self) {
        self.timers.delay=0;
        self.timers.sound=0;
        self.registers.index=0;
        self.registers.sp = SP_OFFSET;
        self.registers.pc = PROGRAM_OFFSET;

    }

    fn load_rom(&mut self,file:Vec<u8>,offset:u8) {

    }
}
