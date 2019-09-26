mod cpu;
mod bus;
mod rom;
mod mapper;
mod ppu;
mod controller;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error;
use sdl2::keyboard::Keycode;
use sdl2::render::{WindowCanvas, Texture};
use sdl2::surface;
use std::time::{SystemTime, Duration, UNIX_EPOCH, Instant};
use sdl2::pixels::PixelFormatEnum;

// NTSC frequency ~1.79 MHz
const NES_CPU_FREQUENCY: f64 = 1.789773;

pub struct Nes {
    cpu: cpu::Cpu,
    frame: u64,
    last_frame: Instant,
    frame_time: Instant
}

impl Nes {
    pub fn new(romfile: &str) -> Result<Nes, Box<dyn Error>> {
        let mapper = mapper::from_file(romfile)?;
        let ppu = ppu::Ppu::new(Rc::clone(&mapper));
        let bus = bus::Bus::new(mapper, ppu);
        let cpu = cpu::Cpu::new(bus);

        Ok(Nes {
            cpu,
            frame: 0,
            last_frame:
            Instant::now(),
            frame_time: Instant::now()
        })
    }

    pub fn powerup(&mut self) {
        self.cpu.powerup();
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.last_frame = Instant::now();
    }

    pub fn update_controller(&mut self, keycode: Keycode, pressed: bool) {
        self.cpu.bus.controller.update(keycode, pressed);
    }

    pub fn run(&mut self, texture: &mut Texture) {
        let elapsed = self.last_frame.elapsed().as_micros() as u64;
        self.last_frame = Instant::now();
        println!("{}", elapsed);
        let cycles = (elapsed as f64*NES_CPU_FREQUENCY) as u64;
        self.cpu.run(cycles);
        if self.cpu.bus.ppu.frame_ready() {
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                self.cpu.bus.ppu.copy_frame(buffer);
            });
            self.frame += 1;
        }
    }
}
