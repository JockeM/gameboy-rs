#![allow(dead_code)]

use crate::gameboy::Gameboy;

#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Zero,
    Subtraction,
    HalfCarry,
    Carry,
}

pub enum Register16 {
    AF,
    BC,
    DE,
    HL,
}

pub enum Register8 {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
}

impl Gameboy {
    pub fn read_flag(&self, flag: Flag) -> bool {
        let f = self.read_u8(Register8::F);
        match flag {
            Flag::Zero => f & 0b1000_0000 != 0,
            Flag::Subtraction => f & 0b0100_0000 != 0,
            Flag::HalfCarry => f & 0b0010_0000 != 0,
            Flag::Carry => f & 0b0001_0000 != 0,
        }
    }

    pub fn write_flag(&mut self, flag: Flag, value: bool) {
        let mut f = self.read_u8(Register8::F);
        match flag {
            Flag::Zero => {
                if value {
                    f |= 0b1000_0000;
                } else {
                    f &= 0b0111_1111;
                }
            }
            Flag::Subtraction => {
                if value {
                    f |= 0b0100_0000;
                } else {
                    f &= 0b1011_1111;
                }
            }
            Flag::HalfCarry => {
                if value {
                    f |= 0b0010_0000;
                } else {
                    f &= 0b1101_1111;
                }
            }
            Flag::Carry => {
                if value {
                    f |= 0b0001_0000;
                } else {
                    f &= 0b1110_1111;
                }
            }
        }
        self.write_u8(Register8::F, f);
    }

    pub fn read_u16(&self, reg: Register16) -> u16 {
        match reg {
            Register16::AF => self.af,
            Register16::BC => self.bc,
            Register16::DE => self.de,
            Register16::HL => self.hl,
        }
    }

    pub fn write_u16(&mut self, reg: Register16, value: u16) {
        match reg {
            Register16::AF => self.af = value,
            Register16::BC => self.bc = value,
            Register16::DE => self.de = value,
            Register16::HL => self.hl = value,
        }
    }

    pub fn read_u8(&self, reg: Register8) -> u8 {
        match reg {
            Register8::A => (self.af >> 8) as u8,
            Register8::F => self.af as u8,
            Register8::B => (self.bc >> 8) as u8,
            Register8::C => self.bc as u8,
            Register8::D => (self.de >> 8) as u8,
            Register8::E => self.de as u8,
            Register8::H => (self.hl >> 8) as u8,
            Register8::L => self.hl as u8,
        }
    }

    pub fn write_u8(&mut self, reg: Register8, value: u8) {
        match reg {
            Register8::A => self.af = (self.af & 0xFF00) | u16::from(value),
            Register8::F => self.af = (self.af & 0x00FF) | (u16::from(value) << 8),
            Register8::B => self.bc = (self.bc & 0xFF00) | u16::from(value),
            Register8::C => self.bc = (self.bc & 0x00FF) | (u16::from(value) << 8),
            Register8::D => self.de = (self.de & 0xFF00) | u16::from(value),
            Register8::E => self.de = (self.de & 0x00FF) | (u16::from(value) << 8),
            Register8::H => self.hl = (self.hl & 0xFF00) | u16::from(value),
            Register8::L => self.hl = (self.hl & 0x00FF) | (u16::from(value) << 8),
        }
    }
}
