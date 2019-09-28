use super::bus;
use rand::prelude::*;
use std::time::{UNIX_EPOCH, SystemTime};

static CPU_INS_CYCLE: [u8; 256] = [
    /*       0 1 2 3 4 5 6 7 8 9 a b c d e f    */
    /*0x00*/ 7,6,2,8,3,3,5,5,3,2,2,2,4,4,6,6,
    /*0x10*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x20*/ 6,6,2,8,3,3,5,5,4,2,2,2,4,4,6,6,
    /*0x30*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x40*/ 6,6,2,8,3,3,5,5,3,2,2,2,3,4,6,6,
    /*0x50*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x60*/ 6,6,2,8,3,3,5,5,4,2,2,2,5,4,6,6,
    /*0x70*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x80*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
    /*0x90*/ 2,6,2,6,4,4,4,4,2,5,2,5,5,5,5,5,
    /*0xA0*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
    /*0xB0*/ 2,5,2,5,4,4,4,4,2,4,2,4,4,4,4,4,
    /*0xC0*/ 2,6,2,8,3,3,5,5,2,2,2,2,4,4,6,6,
    /*0xD0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0xE0*/ 2,6,3,8,3,3,5,5,2,2,2,2,4,4,6,6,
    /*0xF0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
];

const CPU_CARRY_FLAG: u8 = 1 << 0;
const CPU_ZERO_FLAG: u8 = 1 << 1;
const CPU_INT_FLAG: u8 = 1 << 2;
const CPU_DEC_FLAG: u8 = 1 << 3;
const CPU_B4_FLAG: u8 = 1 << 4;
const CPU_B5_FLAG: u8 = 1 << 5;
const CPU_OVF_FLAG: u8 = 1 << 6;
const CPU_NEG_FLAG: u8 = 1 << 7;

const CPU_NMI_VECTOR: u16 = 0xfffa;
const CPU_RESET_VECTOR: u16 = 0xfffc;
const CPU_BRK_VECTOR: u16 = 0xfffe;

trait AddressingMode {
    fn target(cpu: &mut Cpu) -> u16;
    fn load(address: u16, cpu: &mut Cpu) -> u8 {
        cpu.load_u8(address)
    }
    fn store(address: u16, value: u8, cpu: &mut Cpu) {
        cpu.store_u8(address, value);
    }
}

struct ImmediateAddressing;
impl AddressingMode for ImmediateAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let addr = cpu.pc;
        cpu.pc += 1;
        addr
    }
}

struct AccumulatorAddressing;
impl AddressingMode for AccumulatorAddressing {
    fn target(_: &mut Cpu) -> u16 {
        0
    }

    fn load(_: u16, cpu: &mut Cpu) -> u8 {
        cpu.a
    }

    fn store(_: u16, value: u8, cpu: &mut Cpu) {
        cpu.a = value;
    }
}

struct RelativeAddressing;
impl AddressingMode for RelativeAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let addr = cpu.pc;
        cpu.pc += 1;
        addr
    }

    fn store(_: u16, _: u8, _: &mut Cpu) {
        panic!("RelativeAddressing::store");
    }
}

struct ZeroPageAddressing;
impl AddressingMode for ZeroPageAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let offset = cpu.fetch_u8();
        offset as u16
    }
}

struct ZeroPageXAddressing;
impl AddressingMode for ZeroPageXAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let offset = cpu.fetch_u8();
        offset.wrapping_add(cpu.x) as u16
    }
}

struct ZeroPageYAddressing;
impl AddressingMode for ZeroPageYAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let offset = cpu.fetch_u8();
        offset.wrapping_add(cpu.y) as u16
    }
}

struct AbsoluteAddressing;
impl AddressingMode for AbsoluteAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let addr = cpu.fetch_u16();
        addr
    }
}

struct IndirectAddressing;
impl AddressingMode for IndirectAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let addr = cpu.fetch_u16();
        cpu.load_u16(addr)
    }

    fn load(_: u16, _: &mut Cpu) -> u8 {
        panic!("IndirectAddressing::load");
    }

    fn store(_: u16, _: u8, _: &mut Cpu) {
        panic!("IndirectAddressing::store");
    }
}

struct AbsoluteXAddressing;
impl AddressingMode for AbsoluteXAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let addr = cpu.fetch_u16();
        let absx = addr.wrapping_add(cpu.x as u16);
        cpu.handle_page_wrap(addr, absx);
        absx
    }

    fn load(address: u16, cpu: &mut Cpu) -> u8 {
        cpu.add_cycles(cpu.page_cross as u64);
        cpu.load_u8(address)
    }
}

struct AbsoluteYAddressing(u16);
impl AddressingMode for AbsoluteYAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let addr = cpu.fetch_u16();
        let absy = addr.wrapping_add(cpu.y as u16);
        cpu.handle_page_wrap(addr, absy);
        absy
    }

    fn load(address: u16, cpu: &mut Cpu) -> u8 {
        cpu.add_cycles(cpu.page_cross as u64);
        cpu.load_u8(address)
    }
}


struct IndirectXAddressing(u16);
impl AddressingMode for IndirectXAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let offset = cpu.fetch_u8();
        cpu.load_u16(cpu.x.wrapping_add(offset) as u16)
    }
}

struct IndirectYAddressing(u16);
impl AddressingMode for IndirectYAddressing {
    fn target(cpu: &mut Cpu) -> u16 {
        let offset = cpu.fetch_u8() as u16;
        let addr = cpu.load_u16(offset);
        let indy = addr + cpu.y as u16;
        cpu.handle_page_wrap(addr, indy);
        indy
    }

    fn load(address: u16, cpu: &mut Cpu) -> u8 {
        cpu.add_cycles(cpu.page_cross as u64);
        cpu.load_u8(address)
    }
}

pub struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    pc: u16,
    pub bus: bus::Bus,
    cycles: u64,
    prev_nmi: bool,
    page_cross: bool
}

impl Cpu {
    pub fn new(bus: bus::Bus) -> Cpu {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            p: 0x34,
            s: 0xFD,
            pc: 0x400,
            bus,
            cycles: 0,
            prev_nmi: false,
            page_cross: false
        }
    }

    pub fn powerup(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.p = 0x34;
        self.s = 0xFD;
        self.cycles = 0;
        self.prev_nmi = false;

        // fetch reset vector
        self.pc = self.bus.load_u16(CPU_RESET_VECTOR);
    }

    pub fn reset(&mut self) {
        self.p |= CPU_INT_FLAG;
        self.s -= 3;
        self.prev_nmi = false;
        
        // fetch reset vector
        self.pc = self.bus.load_u16(CPU_RESET_VECTOR);
        self.cycles = 0;
    }

    #[inline]
    fn add_cycles(&mut self, cycles: u64) {
        self.cycles = self.cycles.wrapping_add(cycles);
    }

    fn load_u8(&mut self, address: u16) -> u8 {
        self.bus.load_u8(address)
    }

    fn load_u16(&mut self, address: u16) -> u16 {
        self.bus.load_u16(address)
    }

    fn store_u8(&mut self, address: u16, value: u8) {
        if address == 0x4014 {
            // perform OAM dma
            // hackish, but the PPU should continue to run DURING dma
            // side effect: we miss 512 cycles on cycle count, must fix this
            let srcaddr = (value as u16) << 8;
            for addr in srcaddr..srcaddr+256 {
                let value = self.bus.load_u8(addr);
                self.bus.store_u8(0x2004, value);
                self.bus.ppu.run(6);
            }
            self.cycles += (self.cycles % 2) + 1;
        }
        else {
            self.bus.store_u8(address, value);
        }
    }

    fn fetch_u8(&mut self) -> u8 {
        let value = self.bus.load_u8(self.pc);
        self.pc += 1;
        value
    }

    fn fetch_u16(&mut self) -> u16 {
        let value = self.bus.load_u16(self.pc);
        self.pc += 2;
        value
    }

    pub fn step(&mut self) -> u64 {
        let nmi_latch = self.bus.pending_nmi();
        if nmi_latch != self.prev_nmi {
            if nmi_latch {
                self.handle_nmi();
            }
            self.prev_nmi = nmi_latch;
        }

        let opcode = self.fetch_u8();
        self.page_cross = false;

        let cycles = self.cycles;
        match opcode {
            // ADC
            0x69 => self.adc::<ImmediateAddressing>(),
            0x65 => self.adc::<ZeroPageAddressing>(),
            0x75 => self.adc::<ZeroPageXAddressing>(),
            0x6D => self.adc::<AbsoluteAddressing>(),
            0x7D => self.adc::<AbsoluteXAddressing>(),
            0x79 => self.adc::<AbsoluteYAddressing>(),
            0x61 => self.adc::<IndirectXAddressing>(),
            0x71 => self.adc::<IndirectYAddressing>(),

            // AND
            0x29 => self.and::<ImmediateAddressing>(),
            0x25 => self.and::<ZeroPageAddressing>(),
            0x35 => self.and::<ZeroPageXAddressing>(),
            0x2D => self.and::<AbsoluteAddressing>(),
            0x3D => self.and::<AbsoluteXAddressing>(),
            0x39 => self.and::<AbsoluteYAddressing>(),
            0x21 => self.and::<IndirectXAddressing>(),
            0x31 => self.and::<IndirectYAddressing>(),

            // ASL
            0x0A => self.asl::<AccumulatorAddressing>(),
            0x06 => self.asl::<ZeroPageAddressing>(),
            0x16 => self.asl::<ZeroPageXAddressing>(),
            0x0E => self.asl::<AbsoluteAddressing>(),
            0x1E => self.asl::<AbsoluteXAddressing>(),

            // BCC
            0x90 => self.bcc::<RelativeAddressing>(),

            // BCS
            0xB0 => self.bcs::<RelativeAddressing>(),

            // BEQ
            0xF0 => self.beq::<RelativeAddressing>(),

            // BIT
            0x24 => self.bit::<ZeroPageAddressing>(),
            0x2C => self.bit::<AbsoluteAddressing>(),

            // BMI
            0x30 => self.bmi::<RelativeAddressing>(),

            // BNE
            0xD0 => self.bne::<RelativeAddressing>(),

            // BPL
            0x10 => self.bpl::<RelativeAddressing>(),

            // BRK
            0x00 => self.brk(),

            // BVC
            0x50 => self.bvc::<RelativeAddressing>(),

            // BVS
            0x70 => self.bvs::<RelativeAddressing>(),

            // CLC
            0x18 => self.clc(),

            // CLD
            0xD8 => self.cld(),

            // CLI
            0x58 => self.cli(),

            // CLV
            0xB8 => self.clv(),

            // CMP
            0xC9 => self.cmp::<ImmediateAddressing>(),
            0xC5 => self.cmp::<ZeroPageAddressing>(),
            0xD5 => self.cmp::<ZeroPageXAddressing>(),
            0xCD => self.cmp::<AbsoluteAddressing>(),
            0xDD => self.cmp::<AbsoluteXAddressing>(),
            0xD9 => self.cmp::<AbsoluteYAddressing>(),
            0xC1 => self.cmp::<IndirectXAddressing>(),
            0xD1 => self.cmp::<IndirectYAddressing>(),

            // CPX
            0xE0 => self.cpx::<ImmediateAddressing>(),
            0xE4 => self.cpx::<ZeroPageAddressing>(),
            0xEC => self.cpx::<AbsoluteAddressing>(),

            // CPY
            0xC0 => self.cpy::<ImmediateAddressing>(),
            0xC4 => self.cpy::<ZeroPageAddressing>(),
            0xCC => self.cpy::<AbsoluteAddressing>(),

            // DEC
            0xC6 => self.dec::<ZeroPageAddressing>(),
            0xD6 => self.dec::<ZeroPageXAddressing>(),
            0xCE => self.dec::<AbsoluteAddressing>(),
            0xDE => self.dec::<AbsoluteXAddressing>(),

            // DEX
            0xCA => self.dex(),

            // DEY
            0x88 => self.dey(),

            // EOR
            0x49 => self.eor::<ImmediateAddressing>(),
            0x45 => self.eor::<ZeroPageAddressing>(),
            0x55 => self.eor::<ZeroPageXAddressing>(),
            0x4D => self.eor::<AbsoluteAddressing>(),
            0x5D => self.eor::<AbsoluteXAddressing>(),
            0x59 => self.eor::<AbsoluteYAddressing>(),
            0x41 => self.eor::<IndirectXAddressing>(),
            0x51 => self.eor::<IndirectYAddressing>(),

            // INC
            0xE6 => self.inc::<ZeroPageAddressing>(),
            0xF6 => self.inc::<ZeroPageXAddressing>(),
            0xEE => self.inc::<AbsoluteAddressing>(),
            0xFE => self.inc::<AbsoluteXAddressing>(),

            // INX
            0xE8 => self.inx(),

            // INY
            0xC8 => self.iny(),

            // JMP
            0x4C => self.jmp::<AbsoluteAddressing>(),
            0x6C => self.jmp::<IndirectAddressing>(),

            // JSR
            0x20 => self.jsr::<AbsoluteAddressing>(),

            // LDA
            0xA9 => self.lda::<ImmediateAddressing>(),
            0xA5 => self.lda::<ZeroPageAddressing>(),
            0xB5 => self.lda::<ZeroPageXAddressing>(),
            0xAD => self.lda::<AbsoluteAddressing>(),
            0xBD => self.lda::<AbsoluteXAddressing>(),
            0xB9 => self.lda::<AbsoluteYAddressing>(),
            0xA1 => self.lda::<IndirectXAddressing>(),
            0xB1 => self.lda::<IndirectYAddressing>(),

            // LDX
            0xA2 => self.ldx::<ImmediateAddressing>(),
            0xA6 => self.ldx::<ZeroPageAddressing>(),
            0xB6 => self.ldx::<ZeroPageYAddressing>(),
            0xAE => self.ldx::<AbsoluteAddressing>(),
            0xBE => self.ldx::<AbsoluteYAddressing>(),

            // LDY
            0xA0 => self.ldy::<ImmediateAddressing>(),
            0xA4 => self.ldy::<ZeroPageAddressing>(),
            0xB4 => self.ldy::<ZeroPageXAddressing>(),
            0xAC => self.ldy::<AbsoluteAddressing>(),
            0xBC => self.ldy::<AbsoluteXAddressing>(),

            // LSR
            0x4A => self.lsr::<AccumulatorAddressing>(),
            0x46 => self.lsr::<ZeroPageAddressing>(),
            0x56 => self.lsr::<ZeroPageXAddressing>(),
            0x4E => self.lsr::<AbsoluteAddressing>(),
            0x5E => self.lsr::<AbsoluteXAddressing>(),

            // NOP
            0xEA => self.nop(),

            // ORA
            0x09 => self.ora::<ImmediateAddressing>(),
            0x05 => self.ora::<ZeroPageAddressing>(),
            0x15 => self.ora::<ZeroPageXAddressing>(),
            0x0D => self.ora::<AbsoluteAddressing>(),
            0x1D => self.ora::<AbsoluteXAddressing>(),
            0x19 => self.ora::<AbsoluteYAddressing>(),
            0x01 => self.ora::<IndirectXAddressing>(),
            0x11 => self.ora::<IndirectYAddressing>(),

            // PHA
            0x48 => self.pha(),

            // PHP
            0x08 => self.php(),

            // PLA
            0x68 => self.pla(),

            // PLP
            0x28 => self.plp(),

            // ROL
            0x2A => self.rol::<AccumulatorAddressing>(),
            0x26 => self.rol::<ZeroPageAddressing>(),
            0x36 => self.rol::<ZeroPageXAddressing>(),
            0x2E => self.rol::<AbsoluteAddressing>(),
            0x3E => self.rol::<AbsoluteXAddressing>(),

            // ROR
            0x6A => self.ror::<AccumulatorAddressing>(),
            0x66 => self.ror::<ZeroPageAddressing>(),
            0x76 => self.ror::<ZeroPageXAddressing>(),
            0x6E => self.ror::<AbsoluteAddressing>(),
            0x7E => self.ror::<AbsoluteXAddressing>(),

            // RTI
            0x40 => self.rti(),

            // RTS
            0x60 => self.rts(),

            // SBC
            0xE9 => self.sbc::<ImmediateAddressing>(),
            0xE5 => self.sbc::<ZeroPageAddressing>(),
            0xF5 => self.sbc::<ZeroPageXAddressing>(),
            0xED => self.sbc::<AbsoluteAddressing>(),
            0xFD => self.sbc::<AbsoluteXAddressing>(),
            0xF9 => self.sbc::<AbsoluteYAddressing>(),
            0xE1 => self.sbc::<IndirectXAddressing>(),
            0xF1 => self.sbc::<IndirectYAddressing>(),

            // SEC
            0x38 => self.sec(),

            // SED
            0xF8 => self.sed(),

            // SEI
            0x78 => self.sei(),

            // STA
            0x85 => self.sta::<ZeroPageAddressing>(),
            0x95 => self.sta::<ZeroPageXAddressing>(),
            0x8D => self.sta::<AbsoluteAddressing>(),
            0x9D => self.sta::<AbsoluteXAddressing>(),
            0x99 => self.sta::<AbsoluteYAddressing>(),
            0x81 => self.sta::<IndirectXAddressing>(),
            0x91 => self.sta::<IndirectYAddressing>(),

            // STX
            0x86 => self.stx::<ZeroPageAddressing>(),
            0x96 => self.stx::<ZeroPageYAddressing>(),
            0x8E => self.stx::<AbsoluteAddressing>(),

            // STY
            0x84 => self.sty::<ZeroPageAddressing>(),
            0x94 => self.sty::<ZeroPageXAddressing>(),
            0x8C => self.sty::<AbsoluteAddressing>(),

            // TAX
            0xAA => self.tax(),

            // TAY
            0xA8 => self.tay(),

            // TSX
            0xBA => self.tsx(),

            // TXA
            0x8A => self.txa(),

            // TXS
            0x9A => self.txs(),

            // TYA
            0x98 => self.tya(),

            // unofficial opcodes
            // ASO
            0x0F => self.aso::<AbsoluteAddressing>(),
            0x1F => self.aso::<AbsoluteXAddressing>(),
            0x1B => self.aso::<AbsoluteYAddressing>(),
            0x07 => self.aso::<ZeroPageAddressing>(),
            0x17 => self.aso::<ZeroPageXAddressing>(),
            0x03 => self.aso::<IndirectXAddressing>(),
            0x13 => self.aso::<IndirectYAddressing>(),

            // RLA
            0x2F => self.rla::<AbsoluteAddressing>(),
            0x3F => self.rla::<AbsoluteXAddressing>(),
            0x3B => self.rla::<AbsoluteYAddressing>(),
            0x27 => self.rla::<ZeroPageAddressing>(),
            0x37 => self.rla::<ZeroPageXAddressing>(),
            0x23 => self.rla::<IndirectXAddressing>(),
            0x33 => self.rla::<IndirectYAddressing>(),

            // LSE
            0x4F => self.lse::<AbsoluteAddressing>(),
            0x5F => self.lse::<AbsoluteXAddressing>(),
            0x5B => self.lse::<AbsoluteYAddressing>(),
            0x47 => self.lse::<ZeroPageAddressing>(),
            0x57 => self.lse::<ZeroPageXAddressing>(),
            0x43 => self.lse::<IndirectXAddressing>(),
            0x53 => self.lse::<IndirectYAddressing>(),

            // RRA
            0x6F => self.rra::<AbsoluteAddressing>(),
            0x7F => self.rra::<AbsoluteXAddressing>(),
            0x7B => self.rra::<AbsoluteYAddressing>(),
            0x67 => self.rra::<ZeroPageAddressing>(),
            0x77 => self.rra::<ZeroPageXAddressing>(),
            0x63 => self.rra::<IndirectXAddressing>(),
            0x73 => self.rra::<IndirectYAddressing>(),

            // AXS
            0x8F => self.axs::<AbsoluteAddressing>(),
            0x87 => self.axs::<ZeroPageAddressing>(),
            0x97 => self.axs::<ZeroPageYAddressing>(),
            0x83 => self.axs::<IndirectXAddressing>(),

            // LAX
            0xAF => self.lax::<AbsoluteAddressing>(),
            0xBF => self.lax::<AbsoluteYAddressing>(),
            0xA7 => self.lax::<ZeroPageAddressing>(),
            0xB7 => self.lax::<ZeroPageYAddressing>(),
            0xA3 => self.lax::<IndirectXAddressing>(),
            0xB3 => self.lax::<IndirectYAddressing>(),

            // DCM
            0xCF => self.dcm::<AbsoluteAddressing>(),
            0xDF => self.dcm::<AbsoluteXAddressing>(),
            0xDB => self.dcm::<AbsoluteYAddressing>(),
            0xC7 => self.dcm::<ZeroPageAddressing>(),
            0xD7 => self.dcm::<ZeroPageXAddressing>(),
            0xC3 => self.dcm::<IndirectXAddressing>(),
            0xD3 => self.dcm::<IndirectYAddressing>(),

            // INS
            0xEF => self.ins::<AbsoluteAddressing>(),
            0xFF => self.ins::<AbsoluteXAddressing>(),
            0xFB => self.ins::<AbsoluteXAddressing>(),
            0xE7 => self.ins::<ZeroPageAddressing>(),
            0xF7 => self.ins::<ZeroPageXAddressing>(),
            0xE3 => self.ins::<IndirectXAddressing>(),
            0xF3 => self.ins::<IndirectYAddressing>(),

            // ALR
            0x4B => self.alr::<ImmediateAddressing>(),

            // ARR
            0x6B => self.arr::<ImmediateAddressing>(),

            // XAA
            0x8B => self.xaa::<ImmediateAddressing>(),

            // OAL
            0xAB => self.oal::<ImmediateAddressing>(),

            // SAX
            0xCB => self.sax::<ImmediateAddressing>(),

            // NOP
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => self.nop(),

            // SKB
            0x80 | 0x82 | 0xC2 | 0xE2 | 0x89 => self.skb(),
            0x04 | 0x14 | 0x34 | 0x44 | 0x54 | 0x64 | 0x74 | 0xD4 | 0xF4 => self.skb(),

            // SKW
            0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => self.skw(),

            // HLT
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 => self.hlt(),
            0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => self.hlt(),

            // TAS
            0x9B => self.tas::<AbsoluteYAddressing>(),

            // SAY
            0x9C => self.say::<AbsoluteXAddressing>(),

            // XAS
            0x9E => self.xas::<AbsoluteYAddressing>(),

            // AXA
            0x9F => self.axa::<AbsoluteYAddressing>(),
            0x93 => self.axa::<IndirectYAddressing>(),

            // ANC
            0x0B | 0x2B => self.anc::<ImmediateAddressing>(),

            // LAS
            0xBB => self.las::<AbsoluteYAddressing>(),

            0xEB => self.sbc::<ImmediateAddressing>(),

            _ => {}
        }

        self.add_cycles(CPU_INS_CYCLE[opcode as usize] as u64);
        self.cycles - cycles
    }

    fn handle_nmi(&mut self) {
        self.pushw(self.pc);
        self.pushb(self.p);
        self.pc = self.bus.load_u16(0xFFFA);
        self.cycles += 7;
    }

    pub fn run(&mut self, max_cycles: u64) {
        let mut remaining = max_cycles as i64;
        while remaining > 0 {
            let cycles = self.step();
            let mut ppu_cycles = cycles*3;
            remaining -= cycles as i64;
            self.bus.ppu.run(ppu_cycles);
        }
    }

    fn pushb(&mut self, value: u8) {
        self.bus.store_u8(self.s as u16 + 0x100, value);
        self.s = self.s.wrapping_sub(1);
    }

    fn pushw(&mut self, value: u16) {
        self.pushb(((value >> 8) & 0xFF) as u8);
        self.pushb((value & 0xFF) as u8);
    }

    fn popb(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        let v = self.bus.load_u8(self.s as u16 + 0x100);
        v
    }

    fn popw(&mut self) -> u16 {
        self.popb() as u16 | ((self.popb() as u16) << 8)
    }

    fn set_flag(&mut self, flag: u8, set: bool) {
        if set {
            self.p |= flag;
        }
        else {
            self.p &= !flag;
        }
    }

    fn get_flag(&self, flag: u8) -> bool {
        self.p & flag == flag
    }

    fn get_carry(&self) -> u8 {
        self.p & CPU_CARRY_FLAG
    }

    fn set_nz(&mut self, value: u8) -> u8 {
        self.set_flag(CPU_ZERO_FLAG, value == 0);
        self.set_flag(CPU_NEG_FLAG, (value as i8) < 0);
        value
    }

    fn handle_page_wrap(&mut self, from: u16, to: u16) {
        if (from & 0xFF00) != (to & 0xFF00) {
            self.page_cross = true;
        }
    }

    fn adc<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self) as u16;
        let a = self.a as u16;
        let carry = self.get_carry() as u16;
        let res = a.wrapping_add(operand).wrapping_add(carry);
        self.set_flag(CPU_CARRY_FLAG, res > 0xFF);
        self.set_flag(CPU_OVF_FLAG, ((a ^ operand) & 0x80 == 0) && (((a ^ res) & 0x80) == 0x80));
        self.a = self.set_nz((res & 0xFF) as u8);
    }

    fn sbc<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self) as u16;
        let a = self.a as u16;
        let carry = (!self.get_flag(CPU_CARRY_FLAG)) as u16;
        let res = a.wrapping_sub(operand).wrapping_sub(carry);
        self.a = self.set_nz((res & 0xFF) as u8);
        self.set_flag(CPU_OVF_FLAG, ((a ^ operand) & 0x80 != 0) && (((a ^ res) & 0x80) != 0));
        self.set_flag(CPU_CARRY_FLAG, res < 0x100);
    }

    fn and<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a = self.set_nz(self.a & operand);
    }

    fn asl<A: AddressingMode>(&mut self) {
        let addr = A::target(self);
        let operand = A::load(addr, self);
        self.set_flag(CPU_CARRY_FLAG, operand & 0x80 != 0);   
        A::store(addr, self.set_nz(operand << 1), self);
    }

    fn bxx<A: AddressingMode>(&mut self, condition: bool) {
        let offset = (A::load(A::target(self), self) as i8) as i16;
        if condition {
            let newpc = ((self.pc as i16) + offset) as u16;
            self.handle_page_wrap(self.pc, newpc);
            self.pc = newpc;
            self.add_cycles(1 + self.page_cross as u64);
        }
    }

    fn bcc<A: AddressingMode>(&mut self) {
        let cond = !self.get_flag(CPU_CARRY_FLAG);
        self.bxx::<A>(cond);
    }

    fn bcs<A: AddressingMode>(&mut self) {
        let cond = self.get_flag(CPU_CARRY_FLAG);
        self.bxx::<A>(cond);
    }

    fn beq<A: AddressingMode>(&mut self) {
        let cond = self.get_flag(CPU_ZERO_FLAG);
        self.bxx::<A>(cond);
    }

    fn bit<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.set_flag(CPU_ZERO_FLAG, self.a & operand == 0);
        self.set_flag(CPU_OVF_FLAG, operand & CPU_OVF_FLAG != 0);
        self.set_flag(CPU_NEG_FLAG, operand & CPU_NEG_FLAG != 0);
    }

    fn bmi<A: AddressingMode>(&mut self) {
        let cond = self.get_flag(CPU_NEG_FLAG);
        self.bxx::<A>(cond);
    }

    fn bne<A: AddressingMode>(&mut self) {
        let cond = !self.get_flag(CPU_ZERO_FLAG);
        self.bxx::<A>(cond);
    }

    fn bpl<A: AddressingMode>(&mut self) {
        let cond = !self.get_flag(CPU_NEG_FLAG);
        self.bxx::<A>(cond);
    }

    fn brk(&mut self) {
        self.pushw(self.pc + 1);
        self.pushb(self.p | CPU_B4_FLAG | CPU_B5_FLAG);
        self.set_flag(CPU_INT_FLAG, true);
        self.pc = self.bus.load_u16(CPU_BRK_VECTOR);
    }

    fn bvc<A: AddressingMode>(&mut self) {
        let cond = !self.get_flag(CPU_OVF_FLAG);
        self.bxx::<A>(cond);
    }

    fn bvs<A: AddressingMode>(&mut self) {
        let cond = self.get_flag(CPU_OVF_FLAG);
        self.bxx::<A>(cond);
    }

    fn clc(&mut self) {
        self.set_flag(CPU_CARRY_FLAG, false);
    }

    fn cld(&mut self) {
        self.set_flag(CPU_DEC_FLAG, false);
    }

    fn cli(&mut self) {
        self.set_flag(CPU_INT_FLAG, false);
    }

    fn clv(&mut self) {
        self.set_flag(CPU_OVF_FLAG, false);
    }

    fn cmp<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self) as u16;
        let a = self.a as u16;
        let res = a.wrapping_sub(operand);
        self.set_flag(CPU_CARRY_FLAG, (res & 0x100) == 0);
        let _ = self.set_nz(res as u8);
    }

    fn cpx<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self) as u16;
        let x = self.x as u16;
        let res = x.wrapping_sub(operand);
        self.set_flag(CPU_CARRY_FLAG, (res & 0x100) == 0);
        let _ = self.set_nz(res as u8);
    }

    fn cpy<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self) as u16;
        let y = self.y as u16;
        let res = y.wrapping_sub(operand);
        self.set_flag(CPU_CARRY_FLAG, (res & 0x100) == 0);
        let _ = self.set_nz(res as u8);
    }

    fn dec<A: AddressingMode>(&mut self) {
        let addr = A::target(self);
        let operand = A::load(addr, self);
        A::store(addr, self.set_nz(operand.wrapping_sub(1)), self);
    }

    fn dex(&mut self) {
        self.x = self.set_nz(self.x.wrapping_sub(1));
    }

    fn dey(&mut self) {
        self.y = self.set_nz(self.y.wrapping_sub(1));
    }

    fn eor<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a = self.set_nz(self.a ^ operand);
    }

    fn inc<A: AddressingMode>(&mut self) {
        let addr = A::target(self);
        let operand = A::load(addr, self);
        A::store(addr, self.set_nz(operand.wrapping_add(1)), self);
    }

    fn inx(&mut self) {
        self.x = self.set_nz(self.x.wrapping_add(1));
    }

    fn iny(&mut self) {
        self.y = self.set_nz(self.y.wrapping_add(1));
    }

    fn jmp<A: AddressingMode>(&mut self) {
        self.pc = A::target(self);
    }

    fn jsr<A: AddressingMode>(&mut self) {
        let newpc = A::target(self);
        self.pushw(self.pc - 1);
        self.pc = newpc;
    }

    fn lda<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a = self.set_nz(operand);
    }

    fn ldx<A: AddressingMode>(&mut self) {
        let target = A::target(self);
        let operand = A::load(target, self);
        self.x = self.set_nz(operand);
    }

    fn ldy<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.y = self.set_nz(operand);
    }

    fn lsr<A: AddressingMode>(&mut self) {
        let addr = A::target(self);
        let operand = A::load(addr, self);
        self.set_flag(CPU_CARRY_FLAG, operand & 0x1 != 0);
        A::store(addr, self.set_nz(operand >> 1), self);
    }

    fn nop(&mut self) {}

    fn ora<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a = self.set_nz(self.a | operand);
    }

    fn pha(&mut self) {
        self.pushb(self.a);
    }

    fn php(&mut self) {
        self.pushb(self.p | CPU_B5_FLAG | CPU_B4_FLAG);
    }

    fn pla(&mut self) {
        let a = self.popb();
        self.a = self.set_nz(a);
    }

    fn plp(&mut self) {
        self.p = self.popb();
    }

    fn rol<A: AddressingMode>(&mut self) {
        let addr = A::target(self);
        let operand = A::load(addr, self);
        let carry = self.get_carry();
        self.set_flag(CPU_CARRY_FLAG, operand & 0x80 != 0);
        A::store(addr, self.set_nz((operand << 1) | carry), self);
    }

    fn ror<A: AddressingMode>(&mut self) {
        let addr = A::target(self);
        let operand = A::load(addr, self);
        let carry = self.get_carry();
        self.set_flag(CPU_CARRY_FLAG, operand & 1 != 0);
        A::store(addr, self.set_nz((operand >> 1) | (carry << 7)), self);
    }

    fn rti(&mut self) {
        self.p = self.popb();
        self.pc = self.popw();
    }

    fn rts(&mut self) {
        self.pc = self.popw() + 1;
    }

    fn sec(&mut self) {
        self.set_flag(CPU_CARRY_FLAG, true);
    }

    fn sed(&mut self) {
        self.set_flag(CPU_DEC_FLAG, true);
    }

    fn sei(&mut self) {
        self.set_flag(CPU_INT_FLAG, true);
    }

    fn sta<A: AddressingMode>(&mut self) {
        A::store(A::target(self), self.a, self);
    }

    fn stx<A: AddressingMode>(&mut self) {
        A::store(A::target(self), self.x, self);
    }

    fn sty<A: AddressingMode>(&mut self) {
        A::store(A::target(self), self.y, self);
    }

    fn tax(&mut self) {
        self.x = self.set_nz(self.a);
    }

    fn tay(&mut self) {
        self.y = self.set_nz(self.a);
    }

    fn tsx(&mut self) {
        self.x = self.set_nz(self.s);
    }

    fn txa(&mut self) {
        self.a = self.set_nz(self.x);
    }

    fn txs(&mut self) {
        self.s = self.x;
    }

    fn tya(&mut self) {
        self.a = self.set_nz(self.y);
    }

    // unofficial opcodes implementation
    fn aso<A: AddressingMode>(&mut self) {
        // this is horribly inefficient
        self.asl::<A>();
        self.ora::<A>();
    }

    fn rla<A: AddressingMode>(&mut self) {
        // this is horribly inefficient
        self.rol::<A>();
        self.and::<A>();
    }

    fn lse<A: AddressingMode>(&mut self) {
        // this is horribly inefficient
        self.lsr::<A>();
        self.eor::<A>();
    }

    fn rra<A: AddressingMode>(&mut self) {
        // this is horribly inefficient
        self.ror::<A>();
        self.adc::<A>();
    }

    fn axs<A: AddressingMode>(&mut self) {
        A::store(A::target(self), self.a & self.x, self);
    }

    fn lax<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a = operand;
        self.x = self.set_nz(operand);
    }

    fn dcm<A: AddressingMode>(&mut self) {
        self.dec::<A>();
        self.cmp::<A>();
    }

    fn ins<A: AddressingMode>(&mut self) {
        self.inc::<A>();
        self.sbc::<A>();
    }

    fn alr<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a &= operand;
        self.lsr::<AccumulatorAddressing>();
    }

    fn arr<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a &= operand;
        self.ror::<AccumulatorAddressing>();
    }

    fn xaa<A: AddressingMode>(&mut self) {
        self.a = self.x;
        self.and::<ImmediateAddressing>();
    }

    fn oal<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        let val = (self.a | 0xEE) & operand;
        self.a = val;
        self.x = self.set_nz(val);
    }

    fn sax<A: AddressingMode>(&mut self) {

    }

    fn skb(&mut self) {
        self.pc += 1;
    }

    fn skw(&mut self) {
        self.pc += 2;
    }

    fn hlt(&mut self) {
        panic!("hlt");
    }

    fn tas<A: AddressingMode>(&mut self) {
        let addr = self.bus.load_u16(self.pc);
        self.pc += 2;
        let target = addr + self.y as u16;
        self.s = self.a & self.x;
        A::store(target, self.s & (((addr >> 8) & 0xFF)+1) as u8, self);
    }

    fn say<A: AddressingMode>(&mut self) {
        let addr = self.bus.load_u16(self.pc);
        self.pc += 2;
        let target = addr + self.x as u16;
        A::store(target, self.y & ((((addr >> 8) & 0xFF)+1) as u8), self);
    }

    fn xas<A: AddressingMode>(&mut self) {
        let addr = self.bus.load_u16(self.pc);
        self.pc += 2;
        let target = addr + self.y as u16;
        A::store(target, self.x & ((((addr >> 8) & 0xFF)+1) as u8), self);
    }

    fn axa<A: AddressingMode>(&mut self) {

    }

    fn anc<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        self.a = self.set_nz(self.a & operand);
        self.set_flag(CPU_CARRY_FLAG, self.get_flag(CPU_NEG_FLAG));
    }

    fn las<A: AddressingMode>(&mut self) {
        let operand = A::load(A::target(self), self);
        let val = self.s & operand;
        self.s = val;
        self.x = val;
        self.a = self.set_nz(val);
    }
}
