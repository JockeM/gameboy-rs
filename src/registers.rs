#![allow(dead_code)]

use crate::gameboy::Gameboy;

#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Zero,
    Subtraction,
    HalfCarry,
    Carry,
}

impl Flag {
    const fn mask(self) -> u8 {
        match self {
            Self::Zero => 0b1000_0000,
            Self::Subtraction => 0b0100_0000,
            Self::HalfCarry => 0b0010_0000,
            Self::Carry => 0b0001_0000,
        }
    }
}

pub enum Register16 {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
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
    fn write_flags(&mut self, zero: bool, subtraction: bool, half_carry: bool, carry: bool) {
        self.write_u8(
            Register8::F,
            (u8::from(zero) << 7)
                | (u8::from(subtraction) << 6)
                | (u8::from(half_carry) << 5)
                | (u8::from(carry) << 4),
        );
    }

    pub fn read_flag(&self, flag: Flag) -> bool {
        self.read_u8(Register8::F) & flag.mask() != 0
    }

    pub fn write_flag(&mut self, flag: Flag, value: bool) {
        let mut f = self.read_u8(Register8::F);
        let mask = flag.mask();

        if value {
            f |= mask;
        } else {
            f &= !mask;
        }

        self.write_u8(Register8::F, f);
    }

    pub fn read_u16(&self, reg: Register16) -> u16 {
        match reg {
            Register16::AF => self.af,
            Register16::BC => self.bc,
            Register16::DE => self.de,
            Register16::HL => self.hl,
            Register16::SP => self.sp,
            Register16::PC => self.pc,
        }
    }

    pub fn write_u16(&mut self, reg: Register16, value: u16) {
        match reg {
            Register16::AF => self.af = value & 0xFFF0,
            Register16::BC => self.bc = value,
            Register16::DE => self.de = value,
            Register16::HL => self.hl = value,
            Register16::SP => self.sp = value,
            Register16::PC => self.pc = value,
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
            Register8::A => self.af = (self.af & 0x00FF) | (u16::from(value) << 8),
            Register8::F => self.af = (self.af & 0xFF00) | u16::from(value & 0xF0),
            Register8::B => self.bc = (self.bc & 0x00FF) | (u16::from(value) << 8),
            Register8::C => self.bc = (self.bc & 0xFF00) | u16::from(value),
            Register8::D => self.de = (self.de & 0x00FF) | (u16::from(value) << 8),
            Register8::E => self.de = (self.de & 0xFF00) | u16::from(value),
            Register8::H => self.hl = (self.hl & 0x00FF) | (u16::from(value) << 8),
            Register8::L => self.hl = (self.hl & 0xFF00) | u16::from(value),
        }
    }

    pub fn alu_add_a(&mut self, value: u8) {
        let a = self.read_u8(Register8::A);
        let result = a.wrapping_add(value);

        self.write_u8(Register8::A, result);
        self.write_flags(
            result == 0,
            false,
            (a & 0x0F) + (value & 0x0F) > 0x0F,
            u16::from(a) + u16::from(value) > 0xFF,
        );
    }

    pub fn alu_adc_a(&mut self, value: u8) {
        let a = self.read_u8(Register8::A);
        let carry = u8::from(self.read_flag(Flag::Carry));
        let result = a.wrapping_add(value).wrapping_add(carry);

        self.write_u8(Register8::A, result);
        self.write_flags(
            result == 0,
            false,
            (a & 0x0F) + (value & 0x0F) + carry > 0x0F,
            u16::from(a) + u16::from(value) + u16::from(carry) > 0xFF,
        );
    }

    pub fn alu_sub_a(&mut self, value: u8) {
        let a = self.read_u8(Register8::A);
        let result = a.wrapping_sub(value);

        self.write_u8(Register8::A, result);
        self.write_flags(result == 0, true, (a & 0x0F) < (value & 0x0F), a < value);
    }

    pub fn alu_sbc_a(&mut self, value: u8) {
        let a = self.read_u8(Register8::A);
        let carry = u8::from(self.read_flag(Flag::Carry));
        let result = a.wrapping_sub(value).wrapping_sub(carry);

        self.write_u8(Register8::A, result);
        self.write_flags(
            result == 0,
            true,
            (a & 0x0F) < ((value & 0x0F) + carry),
            u16::from(a) < u16::from(value) + u16::from(carry),
        );
    }

    pub fn alu_and_a(&mut self, value: u8) {
        let result = self.read_u8(Register8::A) & value;

        self.write_u8(Register8::A, result);
        self.write_flags(result == 0, false, true, false);
    }

    pub fn alu_xor_a(&mut self, value: u8) {
        let result = self.read_u8(Register8::A) ^ value;

        self.write_u8(Register8::A, result);
        self.write_flags(result == 0, false, false, false);
    }

    pub fn alu_or_a(&mut self, value: u8) {
        let result = self.read_u8(Register8::A) | value;

        self.write_u8(Register8::A, result);
        self.write_flags(result == 0, false, false, false);
    }

    pub fn alu_cp_a(&mut self, value: u8) {
        let a = self.read_u8(Register8::A);
        let result = a.wrapping_sub(value);

        self.write_flags(result == 0, true, (a & 0x0F) < (value & 0x0F), a < value);
    }
}
