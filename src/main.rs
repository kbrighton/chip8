use std::fs::File;
use std::io::Read;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::keyboard::Keycode::H;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

static SP_OFFSET: u16 = 0;
static PROGRAM_OFFSET: u16 = 0x200;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (HEIGHT as u32) * SCALE;
const TICKS_PER_FRAME: usize = 10;
const STACK_SIZE: usize = 16;

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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];


struct Chip8 {
    registers: Registers,
    timers: Timers,
    memory: [u8; 4096],
    screen: [bool; WIDTH * HEIGHT],
    running: bool,
    operand: u16,
    stack: [u16; STACK_SIZE]
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
    fn new() -> Self {
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
            screen: [false; WIDTH * HEIGHT],
            running: true,
            operand: 0,
            stack: [0; STACK_SIZE]
        };
        emu.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        emu
    }
    fn reset(&mut self) {
        self.timers.delay = 0;
        self.timers.sound = 0;
        self.registers.index = 0;
        self.registers.sp = SP_OFFSET;
        self.registers.pc = PROGRAM_OFFSET;
        self.registers.v = [0; 16];
        self.registers.rpl = [0; 16];
        self.running = true;
        self.operand = 0;
        self.screen = [false; WIDTH * HEIGHT];
        self.stack = [0; STACK_SIZE];
    }

    fn push(&mut self, val: u16) {
        self.stack[self.registers.sp as usize] = val;
        self.registers.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.registers.sp -= 1;
        self.stack[self.registers.sp as usize]
    }

    fn load_rom(&mut self, data: &[u8]) {
        let start = PROGRAM_OFFSET as usize;
        let end = (PROGRAM_OFFSET as usize) + data.len();
        self.memory[start..end].copy_from_slice(data);
    }

    fn clock(&mut self) {
        let operation = self.fetch();
        self.execute(operation);
    }

    fn fetch(&mut self) -> u16 {
        let mut operation = 0;
        let top_half = self.memory[self.registers.pc as usize] as u16;
        let bottom_half = self.memory[(self.registers.pc + 1) as usize] as u16;
        self.operand = (top_half << 8) | bottom_half;
        self.registers.pc += 2;

        self.operand
    }

    fn execute(&mut self, operation: u16) {
        let op1 = (operation & 0xF000) >> 12;
        let op2 = (operation & 0x0F00) >> 8;
        let op3 = (operation & 0x00F0) >> 4;
        let op4 = (operation & 0x000F);

        match(op1, op2, op3, op4) {
            (0,0,0,0) => return,
            (0,0,0xE,0) => {
                self.screen = [false; WIDTH * HEIGHT];
            },
            (0,0,0xE,0xE) => {
                let return_addr = self.pop();
                self.registers.pc = return_addr;
            },
            (1, _, _, _) => {
              let nnn = operation & 0xFFF;
              self.registers.pc = nnn;
            },
            (2,_,_,_) => {
                let nnn = operation & 0xFFF;
                self.push(self.registers.pc);
                self.registers.pc = nnn;
            },
            (3,_,_,_) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                if( self.registers.v[x] == nn) {
                    self.registers.pc += 2;
                }
            },
            (4,_,_,_) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                if( self.registers.v[x] != nn) {
                    self.registers.pc += 2;
                }
            },
            (5,_,_,_) => {
                let x = op2 as usize;
                let y = op3 as usize;
                if self.registers.v[x] == self.registers.v[y] {
                    self.registers.pc += 2;
                }
            },
            (6, _, _, _) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                self.registers.v[x] = nn;
            },
            (7, _, _, _) => {
                let x = op2 as usize;
                let nn = (operation & 0xFF) as u8;
                self.registers.v[x] = self.registers.v[x].wrapping_add(nn);
            },
            (8, _, _, 0) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] = self.registers.v[y];
            },
            (8, _, _, 1) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] |= self.registers.v[y];
            },
            (8, _, _, 2) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] &= self.registers.v[y];
            },
            (8, _, _, 3) => {
                let x = op2 as usize;
                let y = op3 as usize;
                self.registers.v[x] ^= self.registers.v[y];
            },
            (8, _, _, 4) => {
                let x = op2 as usize;
                let y = op3 as usize;
                let (new_x, carry) = self.registers.v[x].overflowing_add(self.registers.v[y]);
                let new_f = if carry {
                    1
                } else {
                    0
                };
                self.registers.v[x] = new_x;
                self.registers.v[0xF]= new_f;
            },
            (8, _, _, 5) => {
                let x = op2 as usize;
                let y = op3 as usize;
                let (new_x, borrow) = self.registers.v[x].overflowing_sub(self.registers.v[y]);
                let new_f = if borrow {
                    0
                } else {
                    1
                };
                self.registers.v[x] = new_x;
                self.registers.v[0xF]= new_f;
            },
            (8, _, _, 6) => {
                let x = op2 as usize;
                let lsb = self.registers.v[x] & 1;

                self.registers.v[x] >>= 1;
                self.registers.v[0xF] = lsb;
            },
            (8, _, _, 7) => {
                let x = op2 as usize;
                let y = op3 as usize;
                let (new_x, borrow) = self.registers.v[y].overflowing_sub(self.registers.v[x]);
                let new_f = if borrow {
                    0
                } else {
                    1
                };
                self.registers.v[x] = new_x;
                self.registers.v[0xF]= new_f;
            },

            (8, _, _, 0xE) => {
                let x = op2 as usize;
                let msb = (self.registers.v[x] >> 7) & 1;

                self.registers.v[x] <<= 1;
                self.registers.v[0xF] = msb;
            },
            (9,_,_,0) => {
                let x = op2 as usize;
                let y = op2 as usize;
                if( self.registers.v[x] != self.registers.v[y]) {
                    self.registers.pc += 2;
                }
            },
            (0xA, _, _, _) => {
                let nnn = operation & 0xFFF;
                self.registers.index = nnn;
            },
            (0xB, _, _, _) => {
                let nnn = operation & 0xFFF;
                self.registers.pc = (self.registers.v[0] as u16) + nnn;
            },
            (0xD, _, _, _) => {
                let x_coord = self.registers.v[op2 as usize] as u16;
                let y_coord = self.registers.v[op3 as usize] as u16;

                let rows = op4;

                let mut flip = false;

                for y_line in 0..rows {
                    let addr = self.registers.index + y_line as u16;
                    let pixels = self.memory[addr as usize];

                    for x_line in 0..8 {
                        if(pixels & (0b1000_0000 >> x_line)) != 0 {
                            let x = (x_coord + x_line) as usize % WIDTH;
                            let y = (y_coord + y_line) as usize % HEIGHT;

                            let index = x + WIDTH * y;
                            flip |= self.screen[index];
                            self.screen[index] ^= true;
                        }
                    }
                }
                if flip {
                    self.registers.v[0xF] = 1;
                } else {
                    self.registers.v[0xF] = 0;
                }
            },
            (0xF,_,3,3) => {
                let x = op2 as usize;
                let v = self.registers.v[x] as f32;

                let hundreds = (v/100.0).floor() as u8;
                let tens = ((v / 10.0) % 10.0).floor() as u8;
                let ones = (v % 10.0) as u8;

                self.memory[self.registers.index as usize] = hundreds;
                self.memory[(self.registers.index + 1) as usize] = tens;
                self.memory[(self.registers.index + 2) as usize] = ones;
            }
            (0xF,_, 5,5) => {
                let x = op2 as usize;
                let i = self.registers.index as usize;
                for index in 0..=x {
                    self.memory[i + index] = self.registers.v[index];
                }
            },
            (0xF,_, 6,5) => {
                let x = op2 as usize;
                let i = self.registers.index as usize;
                for index in 0..=x {
                    self.registers.v[index] = self.memory[i + index];
                }
            },
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {:#04x}", operation),
        }
    }

    fn update_timer(&mut self) {
        if self.timers.delay > 0 {
            self.timers.delay -= 1;
        }

        if self.timers.sound > 0 {
            if self.timers.sound == 1 {

            }
            self.timers.sound -= 1;
        }
    }

    fn get_screen_buf(&self) -> &[bool] {
        &self.screen
    }
}

fn main() -> Result<(), String> {
    let mut chip: Chip8 = Chip8::new();
    let mut program = File::open("./test_opcode.ch8").expect("No File Found");
    let mut buffer = Vec::new();

    program.read_to_end(&mut buffer).unwrap();

    chip.load_rom(&buffer);

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Chip8 Emu", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .vulkan()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                _ => {}
            }
        }

        for _ in 0..10 {
            chip.clock();
        }
        chip.update_timer();
        update_screen(&chip, &mut canvas);

    }

    Ok(())
}


fn update_screen(emu: &Chip8, canvas: &mut Canvas<Window>) {
    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();

    let screen_buf = emu.get_screen_buf();
    canvas.set_draw_color(Color::RGB(255,255,255));
    for (i, pixel) in screen_buf.iter().enumerate() {
        if *pixel {
            let x = (i % WIDTH) as u32;
            let y = (i / WIDTH) as u32;

            let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
            canvas.fill_rect(rect).unwrap();
        }
    }
    canvas.present();
}