mod nes;
use std::io::prelude::*;
use std::fs;
use std::time;

extern crate sdl2;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::surface;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let mut nes = nes::Nes::new("roms/super_mario_bros.nes").unwrap();
    let mut elasped: Option<time::Instant> = None;
    let test_status: u8 = 0xFF;
    nes.powerup();

    let sdl_ctx = sdl2::init().unwrap();
    let video = sdl_ctx.video().unwrap();

    let window = video.window("yasnese v0.1", 256*4, 240*4)
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
    let i = 0;
    let mut pause: bool = false;

    let mut last_frame_time = SystemTime::now();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    nes.reset();
                },
                Event::KeyDown { keycode: Some(Keycode::Space), ..} => {
                    pause = !pause;
                },
                Event::KeyDown { keycode: k @ Some(_), .. } => {
                    nes.update_controller(k.unwrap(), true);
                },
                Event::KeyUp { keycode: k @ Some(_), .. } => {
                    nes.update_controller(k.unwrap(), false);
                }
                _ => {}
            }
        }

        if pause {
            continue 'running;
        }

        nes.run(&mut texture);
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
