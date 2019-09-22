mod nes;
use std::io::prelude::*;
use std::fs;
use std::time;

extern crate sdl2;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

fn main() {
    let mapper = nes::mapper::from_file("roms/donkey_kong.nes").unwrap();
    let mut bus = nes::bus::Bus::new(mapper);
    let mut cpu = nes::cpu::Cpu::new(&mut bus);
    let mut elasped: Option<time::Instant> = None;
    let mut test_status: u8 = 0xFF;
    cpu.powerup();

    let sdl_ctx = sdl2::init().unwrap();
    let video = sdl_ctx.video().unwrap();

    let window = video.window("yasnese v0.1", 256*5, 240*5)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().build().unwrap();
    canvas.set_logical_size(256, 240);

    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::ARGB8888, 256, 240).unwrap();
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_ctx.event_pump().unwrap();
    let mut i = 0;
    let mut pause: bool = false;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    cpu.reset();
                },
                Event::KeyDown { keycode: Some(Keycode::Space), ..} => {
                    pause = !pause;
                },
                Event::KeyDown { keycode: k @ Some(_), .. } => {
                    cpu.bus.controller.update(k.unwrap(), true);
                },
                Event::KeyUp { keycode: k @ Some(_), .. } => {
                    cpu.bus.controller.update(k.unwrap(), false);
                }
                _ => {}
            }
        }

        if pause {
            continue 'running;
        }
        // roughly once every scanline
        cpu.run(341);
        if cpu.bus.ppu.take_frame() {
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                cpu.bus.ppu.copy_frame(buffer);
            });
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }
    }
}
