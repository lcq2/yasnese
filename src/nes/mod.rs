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
use std::time::{SystemTime, Duration, UNIX_EPOCH};

// NTSC frequency ~1.79 MHz
const NES_CPU_FREQUENCY: u64 = 1_789_773;

pub struct Nes {
    cpu: cpu::Cpu,
    frame: u64,
    last_frame: u128,
    start_time: u128
}

impl Nes {
    pub fn new(romfile: &str) -> Result<Nes, Box<dyn Error>> {
        let mapper = mapper::from_file(romfile)?;
        let bus = bus::Bus::new(mapper);
        let cpu = cpu::Cpu::new(bus);

        Ok(Nes { cpu, frame: 0, last_frame: 0, start_time: 0 })
    }

    pub fn powerup(&mut self) {
        self.cpu.powerup();
        self.start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn update_controller(&mut self, keycode: Keycode, pressed: bool) {
        self.cpu.bus.controller.update(keycode, pressed);
    }

    pub fn run(&mut self, canvas: &mut WindowCanvas, texture: &mut Texture) {
        self.cpu.run(341*262);
        if self.cpu.bus.ppu.frame_ready() {
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                self.cpu.bus.ppu.copy_frame(buffer);
            });
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
            self.frame += 1;
            let curtime = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let elapsed = curtime - self.last_frame;
            if elapsed < 1_000/30 {
                std::thread::sleep(Duration::from_millis(1_000/30 - elapsed as u64));
            }
            self.last_frame = curtime;
        }
    }
}