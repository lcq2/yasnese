use super::mapper;
use super::ppu;
use super::controller;
use std::rc::Rc;
use std::cell::RefCell;
use byteorder::{ByteOrder, LittleEndian};

pub struct Bus {
    pub ram: [u8; 0x800],
    pub mapper: Rc<RefCell<dyn mapper::Mapper>>,
    pub ppu: ppu::Ppu,
    pub controller: controller::Controller
}

impl Bus {
    pub fn new(mapper: Rc<RefCell<dyn mapper::Mapper>>, ppu: ppu::Ppu) -> Bus {
        Bus {
            ram: [0; 0x800],
            mapper: Rc::clone(&mapper),
            ppu,
            controller: controller::Controller::new()
        }
    }

    pub fn load_u8(&mut self, address: u16) -> u8 {
        if address < 0x2000 {
            return unsafe {*self.ram.get_unchecked((address % 0x800) as usize) };
        }
        else if address < 0x4000 {
            return self.ppu.read_reg(address % 0x08);
        }
        else if address == 0x4016 {
            self.controller.load_u8()
        }
        else if address < 0x4020 {
            return 0;
        }
        else {
            return self.mapper.borrow().load_prg_u8(address);
        }
    }

    pub fn load_u16(&mut self, address: u16) -> u16 {
        self.load_u8(address) as u16 | ((self.load_u8(address+1) as u16) << 8)
    }

    pub fn store_u8(&mut self, address: u16, value: u8) {
        if address < 0x2000 {
            unsafe { *self.ram.get_unchecked_mut((address % 0x800) as usize) = value };
        }
        else if address < 0x4000 {
            self.ppu.write_reg(address % 0x08, value);
        }
        else if address == 0x4016 {
            self.controller.store_u8(value);
        }
        else if address < 0x4020 {
            // APU
        }
        else {
            self.mapper.borrow_mut().store_prg_u8(address, value);
        }
    }

    pub fn load_into_ram(&mut self, data: &[u8], offset: Option<usize>) {
        self.ram[offset.unwrap_or(0)..].copy_from_slice(data);
    }

    pub fn pending_nmi(&self) -> bool {
        self.ppu.pending_nmi()
    }

    pub fn reset(&mut self) {
        self.ppu.reset();
    }
}