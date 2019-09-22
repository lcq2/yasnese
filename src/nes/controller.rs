use sdl2::keyboard::Keycode;

const CONTROLLER_A: usize = 0;
const CONTROLLER_B: usize = 1;
const CONTROLLER_SELECT: usize = 2;
const CONTROLLER_START: usize = 3;
const CONTROLLER_UP: usize = 4;
const CONTROLLER_DOWN: usize = 5;
const CONTROLLER_LEFT: usize = 6;
const CONTROLLER_RIGHT: usize = 7;

pub struct Controller {
    buttons: [bool; 8],
    strobe: bool,
    index: usize
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            buttons: [false; 8],
            strobe: false,
            index: 0
        }
    }

    pub fn store_u8(&mut self, value: u8) {
        self.strobe = value & 1 != 0;
        if self.strobe {
            self.index = CONTROLLER_A;
        }
    }

    pub fn load_u8(&mut self) -> u8 {
        if self.strobe {
            return self.buttons[CONTROLLER_A] as u8;
        }

        if self.index < 8 {
            let state = self.buttons[self.index];
            self.index += 1;
            return state as u8;
        }
        1
    }

    pub fn update(&mut self, keycode: Keycode, pressed: bool) {
        match keycode {
            Keycode::Left => {
                self.buttons[CONTROLLER_LEFT] = pressed;
            },
            Keycode::Right => {
                self.buttons[CONTROLLER_RIGHT] = pressed;
            },
            Keycode::Up => {
                self.buttons[CONTROLLER_UP] = pressed;
            },
            Keycode::Down => {
                self.buttons[CONTROLLER_DOWN] = pressed;
            },
            Keycode::D => {
                self.buttons[CONTROLLER_B] = pressed;
            },
            Keycode::F => {
                self.buttons[CONTROLLER_A] = pressed;
            },
            Keycode::LCtrl => {
                self.buttons[CONTROLLER_START] = pressed;
            },
            Keycode::LAlt => {
                self.buttons[CONTROLLER_SELECT] = pressed;
            },
            _ => {}
        }
    }
}