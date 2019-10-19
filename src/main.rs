mod nes;
use std::io::prelude::*;
use std::fs;
use std::time;
use std::env;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::AudioSubsystem;
use sdl2::audio::AudioSpecDesired;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let mut nes = if args.len() > 1 {
        nes::Nes::new(&args[1])?
    }
    else {
        nes::Nes::new("roms/super_mario_bros_u.nes")?
    };

    let sdl_ctx = sdl2::init()?;
    let video = sdl_ctx.video()?;
    let audio = sdl_ctx.audio()?;

    let audio_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: Some(128)
    };

    let audio_queue = Rc::new(RefCell::new(audio.open_queue::<u8, _>(None, &audio_spec)?));
    audio_queue.borrow().clear();

    let window = video.window("yasnese v0.1", 256*4, 240*4)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().present_vsync().accelerated().build()?;

    canvas.set_logical_size(256, 240);

    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::ARGB8888, 256, 240)?;
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_ctx.event_pump()?;
    let i = 0;
    let mut pause: bool = false;

    nes.powerup();
    nes.reset();
    nes.set_audio_queue(Rc::clone(&audio_queue));
    audio_queue.borrow().resume();

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
        canvas.copy(&texture, None, None)?;
        canvas.present();
    }
    Ok(())
}
