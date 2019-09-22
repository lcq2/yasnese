use std::io::prelude::*;
use std::fs;
use std::io;
use std::io::SeekFrom;
use std::error::Error;
use byteorder::{LittleEndian, ReadBytesExt};

const NES_ROM_SIGNATURE: u32 = 0x1A53454E;
const NES_ROM_MIRRORING: u8 = 1 << 0;
const NES_ROM_HAS_RAM: u8 = 1 << 1;
const NES_ROM_HAS_TRAINER: u8 = 1 << 2;
const NES_ROM_IGNORE_MIRRORING: u8 = 1 << 3;

pub enum Mirroring {
    Vertical,
    Horizontal
}

pub struct NesRom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub has_ram: bool,
    pub mapper_id: u8
}

impl NesRom {
    pub fn new(filename: &str) -> Result<NesRom, Box<dyn Error>> {
        let f = fs::File::open(filename)?;
        let metadata = f.metadata()?;

        // ensure this is a valid nes rom
        if metadata.len() < 16 {
            return Err("invalid header".into());
        }

        let mut reader = io::BufReader::new(f);
        let sig = reader.read_u32::<LittleEndian>()?;
        if sig != NES_ROM_SIGNATURE {
            return Err("invalid signature".into());
        }

        let mut chr_ram = false;

        let prg_sz = reader.read_u8()?;
        let mut chr_sz = reader.read_u8()?;
        let fl6 = reader.read_u8()?;
        let fl7 = reader.read_u8()?;
        let fl8 = reader.read_u8()?;
        let fl9 = reader.read_u8()?;
        let fl10 = reader.read_u8()?;
        let _ = reader.seek(SeekFrom::Current(5))?;

        if chr_sz == 0 {
            chr_ram = true;
            chr_sz = 1;
        }
        let mut prg_rom = vec![0u8; prg_sz as usize * 16384];
        let mut chr_rom = vec![0u8; chr_sz as usize * 8192];
        reader.read_exact(&mut prg_rom)?;

        if !chr_ram {
            reader.read_exact(&mut chr_rom)?;
        }

        let mirroring = if fl6 & NES_ROM_MIRRORING != 0 {
            Mirroring::Vertical
        }
        else {
            Mirroring::Horizontal
        };
        let has_ram = fl6 & NES_ROM_HAS_RAM != 0;
        let mapper_id = ((fl6 & 0xF0) >> 4) | (fl7 & 0xF0);

        Ok(NesRom {
            prg_rom,
            chr_rom,
            mirroring,
            has_ram,
            mapper_id
        })
    }
}