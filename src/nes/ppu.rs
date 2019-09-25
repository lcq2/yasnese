use super::mapper;
use super::rom;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sdl2::surface;

const PPU_SCREEN_BPP: usize = 4;
const PPU_SCREEN_WIDTH: usize = 256;
const PPU_SCREEN_HEIGHT: usize = 240;
const PPU_FRAMEBUFFER_SZ: usize = PPU_SCREEN_WIDTH*PPU_SCREEN_HEIGHT*PPU_SCREEN_BPP;

pub struct Ppu {
    mapper: Rc<RefCell<dyn mapper::Mapper>>,
    ppu_ctrl: u8,
    ppu_mask: u8,
    ppu_status: u8,
    oam_addr: u8,
    oam_data: u8,
    oam: [u8; 256],
    ram: [u8; 0x800],
    palette: [u8; 0x20],
    sec_oam: [u8; 32],
    sec_oam_index: usize,
    sprite_count: usize,
    next_sprite_count: usize,
    vram_addr_incr: u8,
    spr_pattern_table: u16,
    bg_pattern_table: u16,
    sprite_h: u8,
    cycles: u32,
    remaining: u64,
    scanline: u32,
    odd_frame: bool,
    framebuffer: [u8; PPU_FRAMEBUFFER_SZ],
    frame_ready: bool,
    nt_mirror: [u16; 4],
    nt: u8,
    at: u8,
    bg_low: u8,
    bg_high: u8,
    tile_data: u64,
    sp_low: u8,
    sp_high: u8,
    sp_at: u8,
    sp_x: [u8; 8],
    sp_data: [u32; 8],
    sp_prio: [u8; 8],
    sp0_hit: bool,
    read_buffer: u8,
    v: u16,
    t: u16,
    x: u8,
    w: bool,
    frame: u128
}

const PPUCTRL: u16 = 0x0;
const PPUMASK: u16 = 0x1;
const PPUSTATUS: u16 = 0x2;
const OAMADDR: u16 = 0x3;
const OAMDATA: u16 = 0x4;
const PPUSCROLL: u16 = 0x5;
const PPUADDR: u16 = 0x6;
const PPUDATA: u16 = 0x7;

const PPU_CTRL_NMI: u8 = 1 << 7;

const PPU_STATUS_SPRITE_OVF: u8         = 1 << 5;
const PPU_STATUS_SPRITE0_HIT: u8        = 1 << 6;
const PPU_STATUS_VBLANK: u8             = 1 << 7;

const PPU_MASK_GRAYSCALE: u8            = 1 << 0;
const PPU_MASK_SHOW_BACKGROUND_LEFT: u8 = 1 << 1;
const PPU_MASK_SHOW_SPRITE_LEFT: u8     = 1 << 2;
const PPU_MASK_SHOW_BACKGROUND: u8      = 1 << 3;
const PPU_MASK_SHOW_SPRITES: u8         = 1 << 4;
const PPU_MASK_EMPH_RED: u8             = 1 << 5;
const PPU_MASK_EMPH_GREEN: u8           = 1 << 6;
const PPU_MASK_EMPH_BLUE: u8            = 1 << 7;

const OAM_SPRITE_PALETTE: u8            = 0b11;
const OAM_SPRITE_PRIORITY: u8           = 1 << 5;
const OAM_SPRITE_FLIP_H: u8             = 1 << 6;
const OAM_SPRITE_FLIP_V: u8             = 1 << 7;

const PPU_CYCLES_PER_SCANLINE: u32 = 340;
const PPU_VISIBLE_SCANLINES: u32 = 240;
const PPU_POSTRENDER_SCANLINES: u32 = 261;
const PPU_VBLANK_SCANLINE: u32 = 241;

static PPU_PALETTE: [u8; 192] = [
    0x66, 0x66, 0x66,   0x00, 0x2A, 0x88,   0x14, 0x12, 0xA7,
    0x3B, 0x00, 0xA4,   0x5C, 0x00, 0x7E,   0x6E, 0x00, 0x40,
    0x6C, 0x06, 0x00,   0x56, 0x1D, 0x00,   0x33, 0x35, 0x00,
    0x0B, 0x48, 0x00,   0x00, 0x52, 0x00,   0x00, 0x4F, 0x08,
    0x00, 0x40, 0x4D,   0x00, 0x00, 0x00,   0x00, 0x00, 0x00,
    0x00, 0x00, 0x00,   0xAD, 0xAD, 0xAD,   0x15, 0x5F, 0xD9,
    0x42, 0x40, 0xFF,   0x75, 0x27, 0xFE,   0xA0, 0x1A, 0xCC,
    0xB7, 0x1E, 0x7B,   0xB5, 0x31, 0x20,   0x99, 0x4E, 0x00,
    0x6B, 0x6D, 0x00,   0x38, 0x87, 0x00,   0x0C, 0x93, 0x00,
    0x00, 0x8F, 0x32,   0x00, 0x7C, 0x8D,   0x00, 0x00, 0x00,
    0x00, 0x00, 0x00,   0x00, 0x00, 0x00,   0xFF, 0xFE, 0xFF,
    0x64, 0xB0, 0xFF,   0x92, 0x90, 0xFF,   0xC6, 0x76, 0xFF,
    0xF3, 0x6A, 0xFF,   0xFE, 0x6E, 0xCC,   0xFE, 0x81, 0x70,
    0xEA, 0x9E, 0x22,   0xBC, 0xBE, 0x00,   0x88, 0xD8, 0x00,
    0x5C, 0xE4, 0x30,   0x45, 0xE0, 0x82,   0x48, 0xCD, 0xDE,
    0x4F, 0x4F, 0x4F,   0x00, 0x00, 0x00,   0x00, 0x00, 0x00,
    0xFF, 0xFE, 0xFF,   0xC0, 0xDF, 0xFF,   0xD3, 0xD2, 0xFF,
    0xE8, 0xC8, 0xFF,   0xFB, 0xC2, 0xFF,   0xFE, 0xC4, 0xEA,
    0xFE, 0xCC, 0xC5,   0xF7, 0xD8, 0xA5,   0xE4, 0xE5, 0x94,
    0xCF, 0xEF, 0x96,   0xBD, 0xF4, 0xAB,   0xB3, 0xF3, 0xCC,
    0xB5, 0xEB, 0xF2,   0xB8, 0xB8, 0xB8,   0x00, 0x00, 0x00,
    0x00, 0x00, 0x00
];

// we reverse bit planes for sanity reasons
// and because we don't really care if we're using a left shift register or right one
static PPU_PATTERN_REVERSE: [u8; 256] = [
    0x00, 0x80, 0x40, 0xc0, 0x20, 0xa0, 0x60, 0xe0,
    0x10, 0x90, 0x50, 0xd0, 0x30, 0xb0, 0x70, 0xf0,
    0x08, 0x88, 0x48, 0xc8, 0x28, 0xa8, 0x68, 0xe8,
    0x18, 0x98, 0x58, 0xd8, 0x38, 0xb8, 0x78, 0xf8,
    0x04, 0x84, 0x44, 0xc4, 0x24, 0xa4, 0x64, 0xe4,
    0x14, 0x94, 0x54, 0xd4, 0x34, 0xb4, 0x74, 0xf4,
    0x0c, 0x8c, 0x4c, 0xcc, 0x2c, 0xac, 0x6c, 0xec,
    0x1c, 0x9c, 0x5c, 0xdc, 0x3c, 0xbc, 0x7c, 0xfc,
    0x02, 0x82, 0x42, 0xc2, 0x22, 0xa2, 0x62, 0xe2,
    0x12, 0x92, 0x52, 0xd2, 0x32, 0xb2, 0x72, 0xf2,
    0x0a, 0x8a, 0x4a, 0xca, 0x2a, 0xaa, 0x6a, 0xea,
    0x1a, 0x9a, 0x5a, 0xda, 0x3a, 0xba, 0x7a, 0xfa,
    0x06, 0x86, 0x46, 0xc6, 0x26, 0xa6, 0x66, 0xe6,
    0x16, 0x96, 0x56, 0xd6, 0x36, 0xb6, 0x76, 0xf6,
    0x0e, 0x8e, 0x4e, 0xce, 0x2e, 0xae, 0x6e, 0xee,
    0x1e, 0x9e, 0x5e, 0xde, 0x3e, 0xbe, 0x7e, 0xfe,
    0x01, 0x81, 0x41, 0xc1, 0x21, 0xa1, 0x61, 0xe1,
    0x11, 0x91, 0x51, 0xd1, 0x31, 0xb1, 0x71, 0xf1,
    0x09, 0x89, 0x49, 0xc9, 0x29, 0xa9, 0x69, 0xe9,
    0x19, 0x99, 0x59, 0xd9, 0x39, 0xb9, 0x79, 0xf9,
    0x05, 0x85, 0x45, 0xc5, 0x25, 0xa5, 0x65, 0xe5,
    0x15, 0x95, 0x55, 0xd5, 0x35, 0xb5, 0x75, 0xf5,
    0x0d, 0x8d, 0x4d, 0xcd, 0x2d, 0xad, 0x6d, 0xed,
    0x1d, 0x9d, 0x5d, 0xdd, 0x3d, 0xbd, 0x7d, 0xfd,
    0x03, 0x83, 0x43, 0xc3, 0x23, 0xa3, 0x63, 0xe3,
    0x13, 0x93, 0x53, 0xd3, 0x33, 0xb3, 0x73, 0xf3,
    0x0b, 0x8b, 0x4b, 0xcb, 0x2b, 0xab, 0x6b, 0xeb,
    0x1b, 0x9b, 0x5b, 0xdb, 0x3b, 0xbb, 0x7b, 0xfb,
    0x07, 0x87, 0x47, 0xc7, 0x27, 0xa7, 0x67, 0xe7,
    0x17, 0x97, 0x57, 0xd7, 0x37, 0xb7, 0x77, 0xf7,
    0x0f, 0x8f, 0x4f, 0xcf, 0x2f, 0xaf, 0x6f, 0xef,
    0x1f, 0x9f, 0x5f, 0xdf, 0x3f, 0xbf, 0x7f, 0xff
];

impl Ppu {
    pub fn new(mapper: Rc<RefCell<dyn mapper::Mapper>>) -> Ppu {
        let mirroring = mapper.borrow().mirroring();
        Ppu {
            mapper: mapper,
            ppu_ctrl: 0,
            ppu_mask: 0,
            ppu_status: 0,
            oam_addr: 0,
            oam_data: 0,
            oam: [0; 256],
            ram: [0; 0x800],
            palette: [0; 0x20],
            sec_oam: [0; 32],
            sec_oam_index: 0,
            sprite_count: 0,
            next_sprite_count: 0,
            vram_addr_incr: 0,
            spr_pattern_table: 0,
            bg_pattern_table: 0,
            sprite_h: 8,
            cycles: 340,
            remaining: 0,
            scanline: 240,
            odd_frame: false,
            framebuffer: [0; PPU_FRAMEBUFFER_SZ],
            nt_mirror: mirroring,
            frame_ready: false,
            tile_data: 0,
            nt: 0,
            at: 0,
            bg_low: 0,
            bg_high: 0,
            sp_low: 0,
            sp_high: 0,
            sp_at: 0,
            sp_x: [0; 8],
            sp_data: [0; 8],
            sp_prio: [0; 8],
            sp0_hit: false,
            read_buffer: 0,
            v: 0,
            t: 0,
            x: 0,
            w: false,
            frame: 0
        }
    }

    pub fn reset(&mut self) {
        self.cycles = 340;
        self.scanline = 240;
        self.odd_frame = false;
        self.v = 0;
        self.t = 0;
        self.x = 0;
        self.w = false;
        self.frame_ready = false;
        self.sprite_count = 0;
        self.next_sprite_count = 0;
        self.sec_oam_index = 0;
    }
    fn write_oam(&mut self, value: u8) {
        self.oam[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn write_reg(&mut self, address: u16, value: u8) {
        if address == PPUCTRL {
            self.ppu_ctrl = value;

            self.vram_addr_incr =  if ((value >> 2) & 1) == 0 { 1 } else { 32 };
            self.spr_pattern_table = if (value >> 3) & 1 == 0 { 0 } else { 0x1000 };
            self.bg_pattern_table = if (value >> 4) & 1 == 0 { 0 } else { 0x1000 };
            self.sprite_h = if ((value >> 5) & 1) == 0 { 8 } else { 16 };

            self.t = (self.t & 0xF3FF) | (((value & 0b11) as u16) << 10);
        }
        else if address == PPUMASK {
            self.ppu_mask = value;
        }
        else if address == OAMADDR {
            self.oam_addr = value;
        }
        else if address == OAMDATA {
            self.write_oam(value);
        }
        else if address == PPUSCROLL {
            match self.w {
                false => {
                    self.t = (self.t & 0xFFE0) | ((value as u16) >> 3);
                    self.x = value & 0b111;
                }
                true => {
                    self.t = (self.t & 0x8FFF) | (((value as u16) & 0x07) << 12);
                    self.t = (self.t & 0xFC1F) | (((value as u16) & 0xF8) << 2);
                }
            }
            self.w = !self.w;
        }
        else if address == PPUADDR {
            match self.w {
                false => {
                    self.t = (self.t & 0x80FF) | (value as u16 & 0x3F) << 8;
                }
                true => {
                    self.t = (self.t & 0xFF00) | (value as u16);
                    self.v = self.t;
                }
            }
            self.w = !self.w;
        }
        else if address == PPUDATA {
            self.store_u8(self.v, value);
            self.v += self.vram_addr_incr as u16;
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        let index = address & 0x3FF;
        if address < 0x2400 {
            return self.nt_mirror[0] + index;
        }
        else if address < 0x2800 {
            return self.nt_mirror[1] + index;
        }
        else if address < 0x2c00 {
            return self.nt_mirror[2] + index;
        }
        else {
            return self.nt_mirror[3] + index;
        }
    }

    fn load_chr(&self, address: u16) -> u8 {
        self.mapper.borrow().load_chr_u8(address)
    }

    fn load_u8(&self, address: u16) -> u8 {
        let mut addr = address & 0x3FFF;
        if addr < 0x2000 {
            return self.load_chr(addr);
        }
        else if addr < 0x3F00 {
            let mirrored = self.mirror_address(addr) & 0x7FF;
            return self.ram[mirrored as usize];
        }
        else if addr < 0x4000 {
            addr &= 0x1F;
            if addr >= 16 && address % 4 == 0 {
                addr -= 16;
            }
            return self.palette[addr as usize];
        }
        else {
            panic!("Invalid PPU access");
        }
    }

    fn store_u8(&mut self, address: u16, value: u8) {
        let mut addr = address & 0x3FFF;
        if addr < 0x2000 {
            self.mapper.borrow_mut().store_chr_u8(addr, value);
        }
        else if addr < 0x3F00 {
            let mirrored = self.mirror_address(addr) & 0x7FF;
            self.ram[mirrored as usize] = value;
        }
        else if addr < 0x4000 {
            addr &= 0x1F;
            if addr >= 16 && address % 4 == 0 {
                addr -= 16;
            }
            self.palette[addr as usize] = value;
        }
        else {
            panic!("Invalid PPU access: address = {:x}", addr);
        }
    }
    pub fn read_reg(&mut self, address: u16) -> u8 {
        if address == PPUSTATUS {
            self.w = false;
            let status = self.ppu_status;
            self.ppu_status &= !PPU_STATUS_VBLANK;
            return status;
        }
        else if address == PPUDATA {
            let mut value: u8 = self.load_u8(self.v);
            if (self.v & 0x3FFF) < 0x3F00 {
                let tmp = self.read_buffer;
                self.read_buffer = value;
                value = tmp;
            }
            else {
                self.read_buffer = self.load_u8(self.v - 0x1000);
            }
            self.v += self.vram_addr_incr as u16;
            return value;
        }
        0
    }

    pub fn pending_nmi(&self) -> bool {
        ((self.ppu_ctrl & PPU_CTRL_NMI) & (self.ppu_status & PPU_STATUS_VBLANK)) != 0
    }

    #[inline]
    fn show_background(&self) -> bool {
        self.ppu_mask & PPU_MASK_SHOW_BACKGROUND != 0
    }

    #[inline]
    fn show_sprites(&self) -> bool {
        self.ppu_mask & PPU_MASK_SHOW_SPRITES != 0
    }

    #[inline]
    fn rendering_enabled(&self) -> bool {
        return self.show_background() || self.show_sprites();
    }

    fn fetch_nt(&mut self) -> u8 {
        let chr_address = 0x2000 | (self.v & 0xFFF);
        self.load_u8(chr_address)
    }

    fn fetch_at(&self) -> u8 {
        let attr_address = 0x23C0 |
            (self.v & 0x0C00) |
            ((self.v >> 4) & 0x38) |
            ((self.v >> 2) & 0x07);
        let shift = ((self.v >> 4) & 4) | (self.v & 2);
        ((self.load_u8(attr_address) >> (shift as u8)) & 3) << 2
    }

    fn fetch_bg_tile(&self, low: bool) -> usize {
        let fine_y = (self.v >> 12) & 0b111;
        let address = self.bg_pattern_table + self.nt as u16 * 16 + fine_y;
        self.load_chr(if low { address } else { address + 8 }) as usize
    }

    fn fetch_sp_tile(&self, idx: usize, low: bool) -> usize {
        let tile = self.sec_oam[idx+1];
        let mut sprow = self.scanline as u16 - self.sec_oam[idx] as u16;
        if self.sprite_h == 8 {
            if self.sp_at & OAM_SPRITE_FLIP_V != 0 {
                sprow = 7 - sprow;
            }
            let address = self.spr_pattern_table + tile as u16*16 + sprow as u16;
            return self.load_chr(if low { address } else { address + 8 }) as usize;
        }
        else {
            return 0;
        }
    }

    fn fetch_tile(&mut self) {
        match self.cycles % 8 {
            1 => {
                self.nt = self.fetch_nt();
            },
            3 => {
                self.at = self.fetch_at();
            },
            5 => {
                self.bg_low = PPU_PATTERN_REVERSE[self.fetch_bg_tile(true)];
            },
            7 => {
                self.bg_high = PPU_PATTERN_REVERSE[self.fetch_bg_tile(false)];
            },
            0 => {
                let mut data: u32 = 0;
                for i in 0..8 {
                    let bit0 = self.bg_low & 0b1;
                    let bit1 = self.bg_high & 0b1;
                    self.bg_low >>= 1;
                    self.bg_high >>= 1;
                    let color = self.at | (bit1 << 1) | bit0;
                    data |= (color as u32) << 4*i;
                }
                self.tile_data |= (data as u64) << 32;
                self.update_x();
            },
            _ => {}
        }
    }

    fn fetch_cycle(&mut self) {
        self.tile_data >>= 4;
        self.fetch_tile();
    }

    fn pre_render(&mut self) {
        if self.cycles == 1 {
            self.ppu_status &= !(PPU_STATUS_VBLANK |
                PPU_STATUS_SPRITE_OVF | PPU_STATUS_SPRITE0_HIT);
        }

        if self.rendering_enabled() {
            if self.cycles >= 1 && self.cycles <= 256 {
                self.fetch_cycle();
                if self.cycles == 256 {
                    self.update_y();
                }
            }
            else if self.cycles == 257 {
                self.copy_horiz();
            }
            else if self.cycles >= 280 && self.cycles <= 304 {
                self.copy_vert();
            }
            else if self.cycles >= 321 && self.cycles <= 336 {
                self.fetch_cycle();
            }
            else if self.cycles == 337 {
                self.fetch_nt();
            }
            else if self.cycles == 339 {
                self.fetch_nt();
                if self.odd_frame {
                    self.cycles += 1;
                }
            }
        }
    }

    fn sprite_pixel(&mut self) -> u8 {
        if !self.show_sprites() {
            return 0;
        }

        let x = self.cycles - 1;
        let y = self.scanline;

        for i in 0..self.sprite_count {
            let xoff = x as i32 - self.sp_x[i] as i32;
            if xoff >= 0 && xoff < 8 {
                let color = (self.sp_data[i] >> (4*xoff as u32)) & 0x0F;
                if color % 4 == 0 {
                    continue;
                }
                self.sp0_hit = i == 0;
                return color as u8;
            }
        }
        return 0;
    }

    fn put_pixel(&mut self, x: usize, y: usize, palette_index: usize) {
        let offset = (y * PPU_SCREEN_WIDTH + x) * PPU_SCREEN_BPP;
        self.framebuffer[offset] = PPU_PALETTE[palette_index * 3 + 2];
        self.framebuffer[offset+1] = PPU_PALETTE[palette_index * 3 + 1];
        self.framebuffer[offset+2] = PPU_PALETTE[palette_index * 3 + 0];
        self.framebuffer[offset+3] = 0xFF;
    }

    #[inline]
    fn hide_sprite8(&self) -> bool {
        return self.ppu_mask & PPU_MASK_SHOW_SPRITE_LEFT == 0;
    }

    #[inline]
    fn hide_back8(&self) -> bool {
        return self.ppu_mask & PPU_MASK_SHOW_BACKGROUND_LEFT == 0;
    }

    fn background_pixel(&self) -> u8 {
        let color = ((self.tile_data >> self.x as u64*4) & 0b1111) as u8;
        color
    }
    fn visible(&mut self) {
        if self.rendering_enabled() {
            if self.cycles >= 1 && self.cycles <= 256 {
                let x = self.cycles - 1;
                let y = self.scanline;

                let bg_pixel = if x < 8 && self.hide_back8() { 0 } else { self.background_pixel() };
                let sp_pixel = if x < 8 && self.hide_sprite8() { 0 } else { self.sprite_pixel() };

                let sp_transp = sp_pixel & 0b11 == 0;
                let bg_transp = bg_pixel & 0b11 == 0;
                let color = match (bg_transp, sp_transp) {
                    (true, true) => { 0 },
                    (true, false) => { sp_pixel | 0x10 },
                    (false, true) => { bg_pixel },
                    (false, false) => {
                        if self.sp0_hit {
                            self.ppu_status |= PPU_STATUS_SPRITE0_HIT;
                        }
                        sp_pixel
                    }
                };

                let palette_index = self.load_u8(0x3f00 + color as u16) & 0x3F;
                self.put_pixel(x as usize, y as usize, palette_index as usize);
                self.fetch_cycle();
                if self.cycles <= 64 {
                    if self.cycles % 2 == 0 {
                        self.sec_oam[self.sec_oam_index] = 0xFF;
                        self.sec_oam_index = (self.sec_oam_index + 1) & 0x1F;
                    }
                }
                else if self.cycles >= 65 && self.cycles <= 256 {
                    if self.cycles % 3 == 2 {
                        let idx = ((self.cycles - 65)/3) as usize + self.oam_addr as usize;
                        let spy = self.oam[idx*4] as u32;
                        if self.scanline >= spy && self.scanline < (spy+self.sprite_h as u32) {
                            if self.next_sprite_count < 8 {
                                self.sec_oam[self.sec_oam_index..self.sec_oam_index+4]
                                    .copy_from_slice(&self.oam[idx*4..idx*4+4]);
                                self.sec_oam_index += 4;
                                self.next_sprite_count += 1;
                            }
                            else {
                                self.ppu_status |= PPU_STATUS_SPRITE_OVF;
                            }
                        }
                    }
                    if self.cycles == 256 {
                        self.update_y();
                        self.sprite_count = self.next_sprite_count;
                    }
                }
            }
            else if self.cycles >= 257 && self.cycles <= 320 {
                let idx = ((self.cycles - 257)/8) as usize;
                let oam_idx = idx*4;
                if idx < self.next_sprite_count {
                    match self.cycles % 8 {
                        2 => {
                            self.sp_at = self.sec_oam[oam_idx + 2];
                        },
                        3 => {
                            let x = self.sec_oam[oam_idx + 3];
                            self.sp_x[idx] = x;
                        },
                        5 => {
                            self.sp_low = PPU_PATTERN_REVERSE[self.fetch_sp_tile(oam_idx, true)];
                        },
                        7 => {
                            self.sp_high = PPU_PATTERN_REVERSE[self.fetch_sp_tile(oam_idx, false)];
                        },
                        0 => {
                            let mut data: u32 = 0;
                            let fliph = self.sp_at & OAM_SPRITE_FLIP_H != 0;
                            let (mut low, mut high) = if fliph {
                                (PPU_PATTERN_REVERSE[self.sp_low as usize],
                                 PPU_PATTERN_REVERSE[self.sp_high as usize])
                            }
                            else {
                                (self.sp_low, self.sp_high)
                            };
                            let at = (self.sp_at & 0b11) << 2;
                            for i in 0..8 {
                                let bit0 = low & 0b1;
                                let bit1 = high & 0b1;
                                low >>= 1;
                                high >>= 1;
                                let color = at | (bit1 << 1) | bit0;
                                data |= (color as u32) << 4 * i;
                            }
                            self.sp_data[idx] = data;
                            self.sp_prio[idx] = (self.sp_at >> 5) & 0b1;
                        },
                        _ => {}
                    }
                }
                if self.cycles == 257 {
                    self.copy_horiz();
                }
            }
            else if self.cycles >= 321 && self.cycles <= 336 {
                self.fetch_cycle();
            }
            else if self.cycles == 337 || self.cycles == 339 {
                self.fetch_nt();
            }
        }
    }

    fn vblank(&mut self) {
        if self.cycles == 1 {
            self.ppu_status |= PPU_STATUS_VBLANK;
        }
    }

    pub fn run(&mut self, num_cycles: u64) -> bool {
        let mut num_cycles = num_cycles + self.remaining;
        self.remaining = 0;
        while num_cycles > 0 {
            num_cycles -= 1;
            match self.scanline {
                261 => self.pre_render(),
                0..=239 => self.visible(),
                241 if self.cycles == 1 => self.vblank(),
                _ => {}
            }

            self.cycles += 1;
            if self.cycles > PPU_CYCLES_PER_SCANLINE {
                self.cycles = 0;
                self.scanline += 1;
                self.sec_oam_index = 0;
                self.next_sprite_count = 0;
                if self.scanline > PPU_POSTRENDER_SCANLINES {
                    self.odd_frame = !self.odd_frame;
                    self.scanline = 0;
                    self.frame_ready = true;
                    self.remaining = num_cycles;
                    return true;
                }
            }
        }
        return false;
    }

    fn update_x(&mut self) {
        // from nesdev
        // update coarse x
        if (self.v & 0x001F) == 31 {
            self.v &= 0xFFE0;
            self.v ^= 0x0400;
        }
        else {
            self.v += 1;
        }
    }

    fn update_y(&mut self) {
        // from nesdev
        // update coarse and fine y
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000;
        }
        else {
            self.v &= 0x8FFF;
            let mut y = (self.v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.v ^= 0x0800;
            }
            else if y == 31 {
                y = 0;
            }
            else {
                y += 1;
            }
            self.v = (self.v & 0xFC1F) | (y << 5);
        }
    }

    fn extract_y(&self) -> u8 {
        return ((self.v >> 5) & 0b11111) as u8;
    }

    fn copy_horiz(&mut self) {
        self.v = (self.v & 0xFBE0) | (self.t & 0x041F);
    }

    fn copy_vert(&mut self) {
        self.v = (self.v & 0x841F) | (self.t & 0x7BE0);
    }

    pub fn frame_ready(&self) -> bool {
        return self.frame_ready;
    }

    pub fn copy_frame(&mut self, dst: &mut [u8]) {
        dst.copy_from_slice(&self.framebuffer);
        self.frame_ready = false;
    }
}