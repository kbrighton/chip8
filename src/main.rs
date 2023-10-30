mod cpu;
mod timing;

use std::env;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};
use crate::cpu::Chip8;
use crate::timing::{TimedSystem,Timing};

const WIDTH: usize = 128;
const HEIGHT: usize = 64;

const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (HEIGHT as u32) * SCALE;
const TICKS_PER_FRAME: usize = 20;

const LOWRES_WIDTH: usize = 64;
const LOWRES_HEIGHT: usize = 32;

const CPU_SYSTEM: &str = "cpu";
const TIMER_SYSTEM: &str = "timer";
const DISPLAY_SYSTEM: &str = "display";

fn main() {
    let mut chip: Chip8 = Chip8::new();

    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: cargo run path/to/game chiptype");
        return;
    }

    let mut program = File::open(&args[1]).expect("Unable to open file");
    let mut buffer = Vec::new();


    program.read_to_end(&mut buffer).unwrap();

    chip.load_rom(&buffer);
    chip.quirks.get_chip(&args[2]);

    let mut timing = Timing::new(
        Instant::now(),
        vec![
            TimedSystem::new(CPU_SYSTEM, 700),
            TimedSystem::new(TIMER_SYSTEM, 60),
            TimedSystem::new(DISPLAY_SYSTEM, 60),
        ],
    );

    let sdl_context = sdl2::init().unwrap();
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
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                },
                Event::KeyDown{keycode: Some(key), ..} => {
                    if let Some(k) = button_translate(key) {
                        chip.keypress(k, true);
                    }
                },
                Event::KeyUp{keycode: Some(key), ..} => {
                    if let Some(k) = button_translate(key) {
                        chip.keypress(k, false);
                    }
                },
                _ => {}
            }
        }

        // The rest of the game loop goes here...
        let instructions = timing.get_instructions(Instant::now());
        for instruction in instructions {
            match instruction.name {
                CPU_SYSTEM => {
                    for _ in 0..instruction.cycles {
                        chip.clock();
                    }
                },
                TIMER_SYSTEM => {
                    for _ in 0..instruction.cycles {
                        chip.update_timer();
                    }
                },
                DISPLAY_SYSTEM => {
                    for _ in 0..instruction.cycles {
                        update_screen(&chip,&mut canvas);
                    }
                },
                unknown => panic!("Unexpected instruction {}", unknown),
            }
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60)); // 60fps

//        for _ in 0..TICKS_PER_FRAME {
//            chip.clock();
//        }
//        chip.update_timer();
//        update_screen(&chip, &mut canvas);
    }
}

fn update_screen(emu: &Chip8, canvas: &mut Canvas<Window>) {
    canvas.set_draw_color(Color::RGBA(0, 0, 0,255));
    canvas.clear();
    let _width = if emu.get_hires() { WIDTH } else { LOWRES_WIDTH };

    let screen_buf = emu.get_screen_buf();
    canvas.set_draw_color(Color::RGBA(255, 255, 255,255));
    for (i, col) in screen_buf.iter().enumerate() {
        for(j,pixel) in col.iter().enumerate() {
            if *pixel {
                let x = j as u32;
                let y = i as u32;

                let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
                canvas.fill_rect(rect).unwrap();
            }
        }

    }
    canvas.present();
}

fn button_translate(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 =>    Some(0x1),
        Keycode::Num2 =>    Some(0x2),
        Keycode::Num3 =>    Some(0x3),
        Keycode::Num4 =>    Some(0xC),
        Keycode::Q =>       Some(0x4),
        Keycode::W =>       Some(0x5),
        Keycode::E =>       Some(0x6),
        Keycode::R =>       Some(0xD),
        Keycode::A =>       Some(0x7),
        Keycode::S =>       Some(0x8),
        Keycode::D =>       Some(0x9),
        Keycode::F =>       Some(0xE),
        Keycode::Z =>       Some(0xA),
        Keycode::X =>       Some(0x0),
        Keycode::C =>       Some(0xB),
        Keycode::V =>       Some(0xF),
        _ =>                None,
    }
}