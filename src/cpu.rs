use rand::random;
const WIDTH: usize = 128;
const HEIGHT: usize = 64;

const LOWRES_WIDTH: usize = 64;
const LOWRES_HEIGHT: usize = 32;


static SP_OFFSET: u16 = 0;
static PROGRAM_OFFSET: u16 = 0x200;

const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (HEIGHT as u32) * SCALE;
const TICKS_PER_FRAME: usize = 20;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const FONTSET_SIZE: usize = 80;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub(crate) struct Chip8 {
    registers: Registers,
    timers: Timers,
    memory: [u8; 4096],
    screen: [[bool; WIDTH]; HEIGHT],
    operand: u16,
    keys:[bool; NUM_KEYS],
    stack: [u16; STACK_SIZE],
    hires: bool,
    v_blank_wait: bool,
    pub(crate) quirks: Quirks,
}

pub(crate) struct Quirks {
    shift_quirks: bool,
    load_store_quirks: bool,
    clip_quirks: bool,
    jump_quirks: bool,
    logic_quirks: bool,
    v_blank_quirks: bool,
    max_size: u16
}

impl Quirks {
    fn new() -> Self {
        let quirks = Self {
            shift_quirks: false,
            load_store_quirks: false,
            clip_quirks: true,
            jump_quirks: false,
            logic_quirks: true,
            v_blank_quirks: true,
            max_size: 3232
        };
        quirks
    }
    pub(crate) fn get_chip(&mut self, chip: &str) {
        match chip {
            "chip8" => {
                self.shift_quirks = false;
                self.load_store_quirks = false;
                self.clip_quirks = true;
                self.jump_quirks = false;
                self.logic_quirks = true;
                self.v_blank_quirks = true;
                self.max_size = 3232
            },
            "schip" => {
                self.shift_quirks = true;
                self.load_store_quirks = true;
                self.clip_quirks = true;
                self.jump_quirks = true;
                self.logic_quirks = false;
                self.v_blank_quirks = false;
                self.max_size = 3583;
            },
            "xo" => {
                self.shift_quirks = false;
                self.load_store_quirks = false;
                self.clip_quirks = false;
                self.jump_quirks = false;
                self.logic_quirks = false;
                self.v_blank_quirks = false;
                self.max_size = 65024;
            }
            _ => {
                self.shift_quirks = false;
                self.load_store_quirks = false;
                self.clip_quirks = true;
                self.jump_quirks = false;
                self.logic_quirks = true;
                self.v_blank_quirks = true;
                self.max_size = 3232;
            }
        }
    }

}

struct Registers {
    index: u16,
    sp: u16,
    pc: u16,
    v: [u8; 16],
    rpl: [u8; 16],
}

struct Timers {
    delay: u8,
    sound: u8,
}

impl Chip8 {
    pub(crate) fn new() -> Self {
        let mut emu = Self {
            timers: Timers { delay: 0, sound: 0 },
            registers: Registers {
                index: 0,
                sp: SP_OFFSET,
                pc: PROGRAM_OFFSET,
                v: [0; 16],
                rpl: [0; 16],
            },
            memory: [0; 4096],
            screen: [[false; WIDTH]; HEIGHT],
            operand: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            hires: false,
            quirks: Quirks::new(),
            v_blank_wait: false,
        };
        emu.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        emu
    }
    pub(crate) fn get_hires(&self) -> bool {
        self.hires
    }
    fn reset(&mut self) {
        self.timers.delay = 0;
        self.timers.sound = 0;
        self.registers.index = 0;
        self.registers.sp = SP_OFFSET;
        self.registers.pc = PROGRAM_OFFSET;
        self.registers.v = [0; 16];
        self.registers.rpl = [0; 16];
        self.operand = 0;
        self.screen = [[false; WIDTH]; HEIGHT];
        self.stack = [0; STACK_SIZE];
        self.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        self.hires = false;
        self.v_blank_wait = false;
    }

    fn push(&mut self, val: u16) {
        self.stack[self.registers.sp as usize] = val;
        self.registers.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.registers.sp -= 1;
        self.stack[self.registers.sp as usize]
    }

    pub(crate) fn load_rom(&mut self, data: &[u8]) {
        let start = PROGRAM_OFFSET as usize;
        let end = (PROGRAM_OFFSET as usize) + data.len();
        self.memory[start..end].copy_from_slice(data);
    }

    pub(crate) fn keypress(&mut self, index: usize, pressed: bool) {
        self.keys[index] = pressed;
    }

    pub(crate) fn clock(&mut self) {
        if self.v_blank_wait && self.quirks.v_blank_quirks {
            return;
        }
        let operation = self.fetch();
        self.execute(operation);
    }

    fn fetch(&mut self) -> u16 {
        let _operation = 0;
        let top_half = self.memory[self.registers.pc as usize] as u16;
        let bottom_half = self.memory[(self.registers.pc + 1) as usize] as u16;
        self.operand = (top_half << 8) | bottom_half;
        self.registers.pc += 2;

        self.operand
    }

    fn draw_normal(&mut self, x_coord:u16, y_coord:u16, rows:u16) -> bool {
        let width = if self.hires { WIDTH } else { LOWRES_WIDTH };
        let height = if self.hires {HEIGHT} else { LOWRES_HEIGHT};
        let mut flip=false;
        let full_rows = if self.hires { rows} else {rows};
        let full_cols = if self.hires {8} else {8};



        for y_line in 0..full_rows {
            let addr = self.registers.index + y_line;
            let pixels = self.memory[addr as usize];

            for x_line in 0..full_cols {

                let mut source =(pixels & (0b1000_0000 >> x_line)) != 0;

                if self.quirks.clip_quirks {
                    if (x_coord%width as u16)+x_line>=width as u16 || (y_coord%height as u16)+y_line>=height as u16 {
                        source = false;
                    }
                }

                if !source {
                    continue;
                }

                let x = (x_coord + x_line) as usize % width;
                let y = (y_coord + y_line) as usize % height;
                flip |= self.screen[y][x];
                self.screen[y][x] ^= true;

            }
        }
        flip
    }

    fn draw_extended(&mut self, x_coord:u16, y_coord:u16) -> bool {
        let width = if self.hires { WIDTH } else { LOWRES_WIDTH };
        let height = if self.hires {HEIGHT} else { LOWRES_HEIGHT};
        let mut flip=false;
        let rows = 16;

        for y_line in 0..rows {
            for x_byte in 0..2 {
                let addr = self.registers.index + (y_line * 2) + x_byte as u16;
                let pixels = self.memory[addr as usize];

                for x_line in 0..8 {
                    let mut source =(pixels & (0b1000_0000 >> x_line)) != 0;

                    if self.quirks.clip_quirks {
                        if (x_coord%width as u16)+x_line>=width as u16 || (y_coord%height as u16)+y_line>=height as u16 {
                            source = false;
                        }
                    }

                    if !source {
                        continue;
                    }

                    let x = (x_coord + x_line + (x_byte * 8)) as usize % width;
                    let y = (y_coord + y_line) as usize % height;
                    flip |= self.screen[y][x];
                    self.screen[y][x] ^= true;
                }
            }
        }
        flip
    }

    fn execute(&mut self, operation: u16) {
        let op1 = (operation & 0xF000) >> 12;
        let op2 = (operation & 0x0F00) >> 8;
        let op3 = (operation & 0x00F0) >> 4;
        let op4 = operation & 0x000F;


        match (op1, op2, op3, op4) {
            (0, 0, 0, 0) => return,
            (0, 0, 0xC, _) => {
                let length = op4 as usize;
                let _width = if self.hires { WIDTH } else { LOWRES_WIDTH };
                let height = if self.hires { HEIGHT } else {LOWRES_HEIGHT};

                for i in (0..height - length).rev()  {
                    self.screen[i+length] = self.screen[i];
                }

                for i in 0..length {
                    self.screen[i] = [false;WIDTH];
                }
            },
            (0, 0, 0xE, 0) => {
                self.screen = [[false; WIDTH]; HEIGHT];
            },
            (0, 0, 0xE, 0xE) => {
                let return_addr = self.pop();
                self.registers.pc = return_addr;
            },
            (0,0,0xF,0xB) => {
                let _width = if self.hires { WIDTH } else { LOWRES_WIDTH };
                let height = if self.hires { HEIGHT } else {LOWRES_HEIGHT};

                for i in 0..height {
                    let row = self.screen[i];
                    for clean in 0..4 {
                        self.screen[i][clean] = false;
                    }

                    for j in 4.._width {
                        self.screen[i][j] = row[j-4];
                    }
                }
            },
            (0,0,0xF,0xC) => {
                let _width = if self.hires { WIDTH } else { LOWRES_WIDTH };
                let height = if self.hires { HEIGHT } else {LOWRES_HEIGHT};

                for i in 0..height {
                    let row = self.screen[i];
                    for j in 0..(_width -4) {
                        self.screen[i][j] = row[j+4];
                    }

                    for clean in _width-4.._width {
                        self.screen[i][clean] = false;
                    }

                }

            },
            (0,0,0xF,0xE) => {
                self.hires = false;
            },
            (0,0,0xF,0xF) => {
                self.hires = true;
            }
            (0,_,_,_) => return,
            (1, _, _, _) => {
                let nnn = operation & 0xFFF;
                self.registers.pc = nnn;
            }
            (2, _, _, _) => {
                let nnn = operation & 0xFFF;
                self.push(self.registers.pc);
                self.registers.pc = nnn;
            }
            (3, _, _, _) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                if self.registers.v[x] == nn {
                    self.registers.pc += 2;
                }
            }
            (4, _, _, _) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                if self.registers.v[x] != nn {
                    self.registers.pc += 2;
                }
            }
            (5, _, _, _) => {
                let x = op2 as usize;
                let y = op3 as usize;
                if self.registers.v[x] == self.registers.v[y] {
                    self.registers.pc += 2;
                }
            }
            (6, _, _, _) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                self.registers.v[x] = nn;
            }
            (7, _, _, _) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                self.registers.v[x] = self.registers.v[x].wrapping_add(nn);
            }
            (8, _, _, 0) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] = self.registers.v[y];
            }
            (8, _, _, 1) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] |= self.registers.v[y];
                if self.quirks.logic_quirks {
                    self.registers.v[0xF] = 0;
                }
            }
            (8, _, _, 2) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] &= self.registers.v[y];
                if self.quirks.logic_quirks {
                    self.registers.v[0xF] = 0;
                }
            }
            (8, _, _, 3) => {

                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] ^= self.registers.v[y];
                if self.quirks.logic_quirks {
                    self.registers.v[0xF] = 0;
                }
            }
            (8, _, _, 4) => {
                let x = op2 as usize;
                let y = op3 as usize;

                let new_vx = self.registers.v[x] as u16 + self.registers.v[y] as u16;

                self.registers.v[x] = new_vx as u8;
                self.registers.v[0xF] = if new_vx > 255 { 1 } else { 0 };
            }
            (8, _, _, 5) => {
                let x = op2 as usize;
                let y = op3 as usize;

                let (new_vx, borrow) = self.registers.v[x].overflowing_sub(self.registers.v[y]);
                let new_vf = if borrow { 0 } else { 1 };

                self.registers.v[x] = new_vx;
                self.registers.v[0xF] = new_vf;

            }
            (8, _, _, 6) => {
                let x = op2 as usize;
                let y = op3 as usize;


                if !self.quirks.shift_quirks {
                    self.registers.v[x] = self.registers.v[y]
                }

                let lsb = self.registers.v[x] & 1;

                self.registers.v[x] >>= 1;
                self.registers.v[0xF] = lsb;
            }
            (8, _, _, 7) => {
                let x = op2 as usize;
                let y = op3 as usize;
                let (new_vx, borrow) = self.registers.v[y].overflowing_sub(self.registers.v[x]);
                let new_vf = if borrow { 0 } else { 1 };

                self.registers.v[x] = new_vx;
                self.registers.v[0xF] = new_vf;
            }

            (8, _, _, 0xE) => {
                let x = op2 as usize;
                let y = op3 as usize;

                if !self.quirks.shift_quirks {
                    self.registers.v[x] = self.registers.v[y]
                }

                let msb = (self.registers.v[x] >> 7) & 1;
                self.registers.v[x] <<= 1;
                self.registers.v[0xF] = msb;
            }
            (9, _, _, 0) => {
                let x = op2 as usize;
                let y = op3 as usize;

                if self.registers.v[x] != self.registers.v[y] {
                    self.registers.pc += 2;
                }
            }
            (0xA, _, _, _) => {
                let nnn = operation & 0xFFF;
                self.registers.index = nnn;
            }
            (0xB, _, _, _) => {
                let nnn = operation & 0xFFF;

                if self.quirks.jump_quirks {
                    let register:usize = ((nnn >> 8)&0xF) as usize;
                    self.registers.pc = (self.registers.v[register] as u16) + nnn;
                } else {
                    self.registers.pc = (self.registers.v[0] as u16) + nnn;
                }


            }
            (0xC,_,_,_) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                self.registers.v[x] = random::<u8>() & nn;
            }
            (0xD, _, _, _) => {
                let x_coord = self.registers.v[op2 as usize] as u16;
                let y_coord = self.registers.v[op3 as usize] as u16;

                let rows = op4;

                let mut flip = false;
                self.registers.v[0xF]=0;

                if self.hires && rows ==0 {
                    flip = self.draw_extended(x_coord, y_coord);
                } else {
                    flip = self.draw_normal(x_coord,y_coord,rows);
                }

                if flip {
                    self.registers.v[0xF] = 1;
                } else {
                    self.registers.v[0xF] = 0;
                }

                if self.quirks.v_blank_quirks {
                    self.v_blank_wait = true;
                }

            },
            (0xE,_,9,0xE) => {
                let x = op2 as usize;
                let v = self.registers.v[x];
                let key = self.keys[v as usize];
                if key {
                    self.registers.pc +=2;
                }
            },
            (0xE,_,0xA,1) => {
                let x = op2 as usize;
                let v = self.registers.v[x];
                let key = self.keys[v as usize];
                if !key {
                    self.registers.pc +=2;
                }
            },
            (0xF,_,0,7) => {
                let x = op2 as usize;
                self.registers.v[x] = self.timers.delay;
            },
            (0xF,_,0,0xA) => {
                let x = op2 as usize;
                let mut key = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.registers.v[x] = i as u8;
                        key = true;
                        break;
                    }
                }

                if !key {
                    self.registers.pc -=2;
                }
            }
            (0xF,_,1,5) => {
                let x = op2 as usize;
                self.timers.delay = self.registers.v[x];
            },
            (0xF,_,1,8) => {
                let x = op2 as usize;
                self.timers.sound = self.registers.v[x];
            },
            (0xF,_,1,0xE) => {
                let x = op2 as usize;
                self.registers.index += self.registers.v[x] as u16;

            },
            (0xF,_,2,9) => {
                let x = op2 as usize;
                let c = self.registers.v[x] as u16;
                self.registers.index = c * 5;
            },
            (0xF,_,3,0) => {
                let x = op2 as usize;
                let c = self.registers.v[x] as u16;
                self.registers.index = c * 10 + (FONTSET.len() as u16);
            },
            (0xF, _, 3, 3) => {
                let x = op2 as usize;
                let v = self.registers.v[x] as f32;

                let hundreds = (v / 100.0).floor() as u8;
                let tens = ((v / 10.0) % 10.0).floor() as u8;
                let ones = (v % 10.0) as u8;

                self.memory[self.registers.index as usize] = hundreds;
                self.memory[(self.registers.index + 1) as usize] = tens;
                self.memory[(self.registers.index + 2) as usize] = ones;
            },
            (0xF, _, 5, 5) => {
                let x = op2 as usize;
                let i = self.registers.index as usize;
                for index in 0..=x {
                    self.memory[i + index] = self.registers.v[index];
                }
                if !self.quirks.load_store_quirks {
                    self.registers.index = self.registers.index + 1;
                }
            },
            (0xF, _, 6, 5) => {
                let x = op2 as usize;
                let i = self.registers.index as usize;
                for index in 0..=x {
                    self.registers.v[index] = self.memory[i + index];
                }
                if !self.quirks.load_store_quirks {
                    self.registers.index = self.registers.index + 1;
                }
            }
            (0xF, _,7,5) => {
                let source = op2 as usize;
                for counter in 0..source + 1
                {
                    self.registers.rpl[counter] = self.registers.v[counter];
                }
            }
            (0xF, _,8,5) => {
                let source = op2 as usize;
                for counter in 0..source + 1
                {
                    self.registers.v[counter] = self.registers.rpl[counter];
                }
            }
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {:#04x}", operation),
        }
    }

    pub(crate) fn update_timer(&mut self) {
        self.v_blank_wait = false;
        if self.timers.delay > 0 {
            self.timers.delay -= 1;
        }

        if self.timers.sound > 0 {
            if self.timers.sound == 1 {}
            self.timers.sound -= 1;
        }
    }

    pub(crate) fn get_screen_buf(&self) -> &[[bool; WIDTH];HEIGHT] {
        &self.screen
    }
}
