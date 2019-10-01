use sdl2::audio::AudioQueue;
use std::rc::Rc;
use std::cell::RefCell;
use std::option::Option;
use std::borrow::BorrowMut;

const APU_STATUS: u16 = 0x4015;
const APU_FRAME_COUNTER: u16 = 0x4017;

const APU_STATUS_PULSE1: u8 = 1 << 0;
const APU_STATUS_PULSE2: u8 = 1 << 1;
const APU_STATUS_TRIANGLE: u8 = 1 << 2;
const APU_STATUS_NOISE: u8 = 1 << 3;
const APU_STATUS_DMC: u8 = 1 << 4;

const APU_PULSE_DUTY_TABLE_MASK: u8 = 0b1100_0000;
const APU_PULSE_DUTY_TABLE_SHIFT: u8 = 6;

const APU_LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
];

// table for square wave (pulse) duty cycle
const APU_PULSE_DUTY: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1]  // 25% negated
];

const APU_SEQUENCER_MODE0: [u16; 4] = [7457, 14913, 22371, 29828];
const APU_SEQUENCER_MODE1: [u16; 5] = [7457, 14913, 22371, 29829, 37281];

struct Pulse {
    enabled: bool,
    duty_table: usize,
    decay_loop: bool,
    length_enabled: bool,
    length_counter: u8,
    decay_enabled: bool,
    decay_v: u8,
    sweep_counter: u8,
    sweep_timer: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_reload: bool,
    sweep_enabled: bool,
    freq_timer: u16,
    freq_counter: u16,
    duty_counter: u8,
    decay_counter: u8,
    decay_reset_flag: bool,
    decay_hidden_v: u8
}

impl Pulse {
    pub fn new() -> Pulse {
        Pulse {
            enabled: false,
            duty_table: 0,
            decay_loop: false,
            length_enabled: false,
            length_counter: 0,
            decay_enabled: false,
            decay_v: 0,
            sweep_counter: 0,
            sweep_timer: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_enabled: false,
            freq_timer: 0,
            freq_counter: 0,
            duty_counter: 0,
            decay_counter: 0,
            decay_reset_flag: false,
            decay_hidden_v: 0
        }
    }

    pub fn reset(&mut self) {
        *self = Pulse::new();
    }

    pub fn write4000(&mut self, value: u8) {
        self.duty_table = ((value & 0b1100_0000) >> 6) as usize;
        self.decay_loop = value & 0b0010_0000 != 0;
        self.length_enabled = !self.decay_loop;
        self.decay_enabled = value & 0b0001_0000 == 0;
        self.decay_v = value & 0b0000_1111;
    }

    pub fn write4001(&mut self, value: u8) {
        self.sweep_timer = (value & 0b0111_0000) >> 4;
        self.sweep_negate = value & 0b0000_1000 != 0;
        self.sweep_shift = value & 0b0000_0111;
        self.sweep_reload = true;
        self.sweep_enabled = (value & 0b1000_0000) != 0 && self.sweep_shift != 0;
    }

    pub fn write4002(&mut self, value: u8) {
        self.freq_timer = (self.freq_timer & 0xFF00) | value as u16;
    }

    pub fn write4003(&mut self, value: u8) {
        self.freq_timer = (self.freq_timer & 0x00FF) | (((value & 0b111) as u16) << 8);
        if self.enabled {
            let idx = ((value & 0b1111_1000) >> 3) as usize;
            self.length_counter = APU_LENGTH_TABLE[idx];
        }
        self.freq_counter = self.freq_timer;
        self.duty_counter = 0;
        self.decay_reset_flag = true;
    }

    pub fn set_channel_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !self.enabled {
            self.length_counter = 0;
        }
    }

    pub fn step(&mut self) {
        if self.freq_counter > 0 {
            self.freq_counter -= 1;
        }
        else {
            self.freq_counter = self.freq_timer;
            self.duty_counter = (self.duty_counter + 1) & 7;
        }
    }

    pub fn decay(&mut self) {
        if self.decay_reset_flag {
            self.decay_reset_flag = false;
            self.decay_hidden_v = 0xF;
            self.decay_counter = self.decay_v;
        }
        else {
            if self.decay_counter > 0 {
                self.decay_counter -= 1;
            }
            else {
                self.decay_counter = self.decay_v;
                if self.decay_hidden_v > 0 {
                    self.decay_hidden_v -= 1;
                }
                else if self.decay_loop {
                    self.decay_hidden_v = 0xF;
                }
            }
        }
    }

    pub fn sweep(&mut self) {
        if self.sweep_reload {
            self.sweep_counter = self.sweep_timer;
            self.sweep_reload = false;
        }
        else if self.sweep_counter > 0 {
            self.sweep_counter -= 1;
        }
        else {
            self.sweep_counter = self.sweep_timer;
            if self.sweep_enabled && !self.sweep_silence() {
                if self.sweep_negate {
                    self.freq_timer -= (self.freq_timer >> self.sweep_shift as u16) + 1;
                }
                else {
                    self.freq_timer += (self.freq_timer >> self.sweep_shift as u16);
                }
            }
        }
    }

    pub fn length(&mut self) {
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn sweep_silence(&self) -> bool {
        if self.freq_timer < 8 {
            return true;
        }
        else if !self.sweep_negate &&
            self.freq_timer + (self.freq_timer + (self.freq_timer >> (self.sweep_shift as u16))) >= 0x800 {
            return true;
        }
        else {
            return false;
        }
    }

    pub fn out(&mut self) -> u8 {
        if APU_PULSE_DUTY[self.duty_table][self.duty_counter as usize] == 1 && self.length_counter != 0 && !self.sweep_silence() {
            return if self.decay_enabled { self.decay_hidden_v } else { self.decay_v };
        }
        return 0;
    }
}

struct Triangle {
    enabled: bool
}

impl Triangle {
    pub fn new() -> Triangle {
        Triangle {
            enabled: false
        }
    }
}

struct Noise {
    enabled: bool
}

impl Noise {
    pub fn new() -> Noise {
        Noise {
            enabled: false
        }
    }
}

struct DMC {
    enabled: bool
}

impl DMC {
    pub fn new() -> DMC {
        DMC {
            enabled: false
        }
    }
}

pub struct Apu {
    audio_queue: Option<Rc<RefCell<AudioQueue<u8>>>>,
    pulse1: Pulse,
    pulse2: Pulse,
    sequencer_mode: u8,
    irq_enabled: bool,
    irq_pending: bool,
    next_seq_phase: usize,
    sequencer_counter: u16,
    cycle: u64,
    apu_cycle: u64,
    accum: f64,
    out_buf: [u8; 735],
    out_index: usize,
    audio_ready: bool
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            audio_queue: None,
            pulse1: Pulse::new(),
            pulse2: Pulse::new(),
            sequencer_mode: 0,
            irq_enabled: false,
            irq_pending: false,
            next_seq_phase: 0,
            sequencer_counter: 0,
            cycle: 0,
            apu_cycle: 0,
            accum: 0.0,
            out_buf: [0; 735],
            out_index: 0,
            audio_ready: false
        }
    }

    pub fn reset(&mut self) {
        self.pulse1.reset();
        self.pulse2.reset();
        self.cycle = 0;
    }

    pub fn write4015(&mut self, value: u8) {
        self.pulse1.set_channel_enabled(value & 0b0000_0001 != 0);
        self.pulse2.set_channel_enabled(value & 0b0000_0010 != 0);
    }

    fn quarter_clock(&mut self) {
        self.pulse1.decay();
        self.pulse2.decay();
    }

    fn half_clock(&mut self) {
        self.pulse1.sweep();
        self.pulse1.length();
        self.pulse2.sweep();
        self.pulse2.length();
    }

    pub fn write4000(&mut self, value: u8) {
        self.pulse1.write4000(value);
    }

    pub fn write4001(&mut self, value: u8) {
        self.pulse1.write4001(value);
    }

    pub fn write4002(&mut self, value: u8) {
        self.pulse1.write4002(value);
    }

    pub fn write4003(&mut self, value: u8) {
        self.pulse1.write4003(value);
    }

    pub fn write4017(&mut self, value: u8) {
        self.sequencer_mode = (value & 0b1000_0000) >> 7;
        self.irq_enabled = (value & 0b0100_0000) == 0;
        self.next_seq_phase = 0;
        self.sequencer_counter = if self.sequencer_mode == 0 {
            APU_SEQUENCER_MODE0[0]
        }
        else {
            APU_SEQUENCER_MODE1[0]
        };

        if self.sequencer_mode == 1 {
            self.quarter_clock();
            self.half_clock();
        }
        if !self.irq_enabled {
            self.irq_pending = false;
        }
    }

    fn mode0(&mut self) -> u16 {
        match self.next_seq_phase {
            0 | 2 => {
                self.quarter_clock();
            },
            1 => {
                self.quarter_clock();
                self.half_clock();
            },
            3 => {
                self.quarter_clock();
                self.half_clock();
                if self.irq_enabled {
                    self.irq_pending = true;
                }
            },
            _ => {}
        };
        self.next_seq_phase += 1;
        if self.next_seq_phase == 4 {
            self.next_seq_phase = 0;
        }
        APU_SEQUENCER_MODE0[self.next_seq_phase]
    }

    fn mode1(&mut self) -> u16 {
        match self.next_seq_phase {
            0 | 2 => {
                self.quarter_clock();
            },
            1 | 3 => {
                self.quarter_clock();
                self.half_clock();
            },
            _ => {}
        };
        self.next_seq_phase +=1 ;
        if self.next_seq_phase == 5 {
            self.next_seq_phase = 0;
        }
        APU_SEQUENCER_MODE1[self.next_seq_phase]
    }

    fn sequencer(&mut self) {
        if self.sequencer_counter > 0 {
            self.sequencer_counter -= 1;
        }
        else {
            self.sequencer_counter = match self.sequencer_mode {
                0 => {
                    self.mode0()
                },
                1 => {
                    self.mode1()
                },
                _ => 0
            };
        }
    }

    fn step(&mut self) {
        self.cycle += 1;
        if self.cycle % 2 == 0 {
            self.apu_cycle += 1;
            self.pulse1.step();
            self.pulse2.step();
        }
        self.sequencer();
        if self.apu_cycle % 40 == 0 {
            let mut sample: u8 = 0;
            if self.pulse1.enabled {
                sample = self.pulse1.out();
            }
            if self.pulse2.enabled {
                sample += self.pulse2.out();
            }
            self.out_buf[self.out_index] = sample;
            self.out_index += 1;
            if self.out_index == self.out_buf.len() {
                if let Some(x) = &mut self.audio_queue {
                    let x = x.borrow();
                    x.queue(&self.out_buf);
                }
                self.out_index = 0;
                self.audio_ready = true;
            }
        }
    }

    pub fn set_audio_queue(&mut self, audio_queue: Rc<RefCell<AudioQueue<u8>>>) {
        self.audio_queue = Some(Rc::clone(&audio_queue));
    }

    pub fn run(&mut self, cycles: u64) {
        let mut cycles = cycles;
        while cycles > 0 {
            self.step();
            cycles -= 1;
        }
    }

    pub fn audio_ready(&self) -> bool {
        self.audio_ready
    }

    pub fn take_audio(&mut self) -> &[u8] {
        self.audio_ready = false;
        &self.out_buf
    }
}