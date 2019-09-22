use super::rom;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error;

pub trait Mapper {
    fn load_prg_u8(&self, address: u16) -> u8;
    fn load_chr_u8(&self, address: u16) -> u8;
    fn store_prg_u8(&mut self, address: u16, value: u8);
    fn store_chr_u8(&mut self, address: u16, value: u8);
    fn mirroring(&self) -> [u16; 4];
}

pub fn from_file(filename: &str) -> Result<Rc<RefCell<dyn Mapper>>, Box<dyn Error>> {
    let rom = rom::NesRom::new(filename)?;
    match rom.mapper_id {
        0 => Ok(Rc::new(RefCell::new(Mapper0 { ram: [0; 0x2000], rom: Box::new(rom) }))),
        _ => Err("Invalid mapper id".into())
    }
}

struct Mapper0 {
    ram: [u8; 0x2000],
    pub rom: Box<rom::NesRom>
}

impl Mapper for Mapper0 {
    fn load_prg_u8(&self, address: u16) -> u8 {
        if address < 0x8000 {
            return unsafe { *self.ram.get_unchecked((address & 0x1FFF) as usize) };
        }
        else if self.rom.prg_rom.len() > 16384 {
            return unsafe { *self.rom.prg_rom.get_unchecked((address & 0x7FFF) as usize) };
        }
        else {
            return unsafe { *self.rom.prg_rom.get_unchecked((address & 0x3FFF) as usize) };
        }
    }
    fn load_chr_u8(&self, address: u16) -> u8 {
        unsafe { *self.rom.chr_rom.get_unchecked(address as usize) }
    }

    fn store_prg_u8(&mut self, address: u16, value: u8) {
        if address < 0x8000 {
            unsafe { *self.ram.get_unchecked_mut((address & 0x1FFF) as usize) = value };
        }
        else {
            panic!("Mapper0::store_prg_u8");
        }
    }

    fn store_chr_u8(&mut self, address: u16, value: u8) {
//      panic!("Mapper0::store_chr_u8");
        unsafe { *self.rom.chr_rom.get_unchecked_mut(address as usize) = value; }
    }

    fn mirroring(&self) -> [u16; 4] {
        match self.rom.mirroring {
            rom::Mirroring::Horizontal => { [0, 0, 0x400, 0x400] },
            rom::Mirroring::Vertical => { [0, 0x400, 0, 0x400] }
        }
    }
}