#![allow(dead_code)]

use crate::ppu::{Ppu, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::registers::*;

use minifb::{Key, Scale, Window, WindowOptions};

use std::fs;
use std::io;
use std::path::Path;

pub struct Gameboy {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16,

    pub mem: [u8; 0x10000],
    pub ppu: Ppu,
    pub cycles: u64,
    pub halted: bool,
    pub stopped: bool,
    pub interrupts_enabled: bool,
}

impl Gameboy {
    pub fn load_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let rom_bytes = fs::read(path)?;
        Ok(Self::load(&rom_bytes))
    }

    pub fn load(rom_bytes: &[u8]) -> Self {
        let mut mem = [0; 0x10000];
        mem[..rom_bytes.len()].copy_from_slice(rom_bytes);

        Self {
            af: 0,
            bc: 0,
            de: 0,
            hl: 0,
            sp: 0,
            pc: 0x100,
            mem,
            ppu: Ppu::new(),
            cycles: 0,
            halted: false,
            stopped: false,
            interrupts_enabled: false,
        }
    }

    pub fn run(&mut self) -> Result<(), minifb::Error> {
        let mut window = Window::new(
            "gameboy-rs",
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            WindowOptions {
                scale: Scale::X4,
                ..WindowOptions::default()
            },
        )?;

        while window.is_open() && !window.is_key_down(Key::Escape) {
            self.ppu.render_checkerboard();
            window.update_with_buffer(&self.ppu.framebuffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;
        }

        Ok(())
    }

    pub fn execute(&mut self) {
        let opcode = self.next_u8();

        match opcode {
            0x00 => {}
            0x01 => {
                let nn = self.next_u16();
                self.write_u16(Register16::BC, nn);
            }
            0x02 => {
                let bc = self.read_u16(Register16::BC);
                self.write_u8_addr(bc, self.read_u8(Register8::A));
            }
            0x03 => {
                let bc = self.read_u16(Register16::BC);
                self.write_u16(Register16::BC, bc.wrapping_add_signed(1));
            }
            0x04 => {
                self.inc_r8_operand(0);
            }
            0x05 => {
                self.dec_r8_operand(0);
            }
            0x06 => {
                let n = self.next_u8();
                self.write_u8(Register8::B, n);
            }
            0x07 => {
                self.rlca();
            }
            0x08 => {
                let address = self.next_u16();
                self.write_u16_addr(address, self.sp);
            }
            0x09 => {
                self.add_hl(self.read_u16(Register16::BC));
            }
            0x0A => {
                let bc = self.read_u16(Register16::BC);
                let value = self.read_u8_addr(bc);
                self.write_u8(Register8::A, value);
            }
            0x0B => {
                let bc = self.read_u16(Register16::BC);
                self.write_u16(Register16::BC, bc.wrapping_sub(1));
            }
            0x0C => {
                self.inc_r8_operand(1);
            }
            0x0D => {
                self.dec_r8_operand(1);
            }
            0x0E => {
                let n = self.next_u8();
                self.write_u8(Register8::C, n);
            }
            0x0F => {
                self.rrca();
            }
            0x10 => {
                self.next_u8();
                self.stopped = true;
            }
            0x11 => {
                let nn = self.next_u16();
                self.write_u16(Register16::DE, nn);
            }
            0x12 => {
                let de = self.read_u16(Register16::DE);
                self.write_u8_addr(de, self.read_u8(Register8::A));
            }
            0x13 => {
                let de = self.read_u16(Register16::DE);
                self.write_u16(Register16::DE, de.wrapping_add(1));
            }
            0x14 => {
                self.inc_r8_operand(2);
            }
            0x15 => {
                self.dec_r8_operand(2);
            }
            0x16 => {
                let n = self.next_u8();
                self.write_u8(Register8::D, n);
            }
            0x17 => {
                self.rla();
            }
            0x18 => {
                let offset = self.next_u8();
                self.jump_relative(offset);
            }
            0x19 => {
                self.add_hl(self.read_u16(Register16::DE));
            }
            0x1A => {
                let de = self.read_u16(Register16::DE);
                let value = self.read_u8_addr(de);
                self.write_u8(Register8::A, value);
            }
            0x1B => {
                let de = self.read_u16(Register16::DE);
                self.write_u16(Register16::DE, de.wrapping_sub(1));
            }
            0x1C => {
                self.inc_r8_operand(3);
            }
            0x1D => {
                self.dec_r8_operand(3);
            }
            0x1E => {
                let n = self.next_u8();
                self.write_u8(Register8::E, n);
            }
            0x1F => {
                self.rra();
            }
            0x20 => {
                let offset = self.next_u8();
                self.jump_relative_if(!self.read_flag(Flag::Zero), offset);
            }
            0x21 => {
                let nn = self.next_u16();
                self.write_u16(Register16::HL, nn);
            }
            0x22 => {
                let hl = self.read_u16(Register16::HL);
                self.write_u8_addr(hl, self.read_u8(Register8::A));
                self.write_u16(Register16::HL, hl.wrapping_add(1));
            }
            0x23 => {
                let hl = self.read_u16(Register16::HL);
                self.write_u16(Register16::HL, hl.wrapping_add(1));
            }
            0x24 => {
                self.inc_r8_operand(4);
            }
            0x25 => {
                self.dec_r8_operand(4);
            }
            0x26 => {
                let n = self.next_u8();
                self.write_u8(Register8::H, n);
            }
            0x27 => {
                self.daa();
            }
            0x28 => {
                let offset = self.next_u8();
                self.jump_relative_if(self.read_flag(Flag::Zero), offset);
            }
            0x29 => {
                self.add_hl(self.read_u16(Register16::HL));
            }
            0x2A => {
                let hl = self.read_u16(Register16::HL);
                let value = self.read_u8_addr(hl);
                self.write_u8(Register8::A, value);
                self.write_u16(Register16::HL, hl.wrapping_add(1));
            }
            0x2B => {
                let hl = self.read_u16(Register16::HL);
                self.write_u16(Register16::HL, hl.wrapping_sub(1));
            }
            0x2C => {
                self.inc_r8_operand(5);
            }
            0x2D => {
                self.dec_r8_operand(5);
            }
            0x2E => {
                let n = self.next_u8();
                self.write_u8(Register8::L, n);
            }
            0x2F => {
                self.cpl();
            }
            0x30 => {
                let offset = self.next_u8();
                self.jump_relative_if(!self.read_flag(Flag::Carry), offset);
            }
            0x31 => {
                self.sp = self.next_u16();
            }
            0x32 => {
                let hl = self.read_u16(Register16::HL);
                self.write_u8_addr(hl, self.read_u8(Register8::A));
                self.write_u16(Register16::HL, hl.wrapping_sub(1));
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
            }
            0x34 => {
                self.inc_r8_operand(6);
            }
            0x35 => {
                self.dec_r8_operand(6);
            }
            0x36 => {
                let n = self.next_u8();
                self.write_u8_addr(self.read_u16(Register16::HL), n);
            }
            0x37 => {
                self.scf();
            }
            0x38 => {
                let offset = self.next_u8();
                self.jump_relative_if(self.read_flag(Flag::Carry), offset);
            }
            0x39 => {
                self.add_hl(self.sp);
            }
            0x3A => {
                let hl = self.read_u16(Register16::HL);
                let value = self.read_u8_addr(hl);
                self.write_u8(Register8::A, value);
                self.write_u16(Register16::HL, hl.wrapping_sub(1));
            }
            0x3B => {
                self.sp = self.sp.wrapping_sub(1);
            }
            0x3C => {
                self.inc_r8_operand(7);
            }
            0x3D => {
                self.dec_r8_operand(7);
            }
            0x3E => {
                let n = self.next_u8();
                self.write_u8(Register8::A, n);
            }
            0x3F => {
                self.ccf();
            }
            0x40..=0x75 | 0x77..=0x7F => {
                let destination = (opcode >> 3) & 0b111;
                let source = opcode & 0b111;
                let value = self.read_r8_operand(source);
                self.write_r8_operand(destination, value);
            }
            0x76 => {
                self.halted = true;
            }
            0x80..=0x87 => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_add_a(value);
            }
            0x88..=0x8F => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_adc_a(value);
            }
            0x90..=0x97 => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_sub_a(value);
            }
            0x98..=0x9F => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_sbc_a(value);
            }
            0xA0..=0xA7 => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_and_a(value);
            }
            0xA8..=0xAF => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_xor_a(value);
            }
            0xB0..=0xB7 => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_or_a(value);
            }
            0xB8..=0xBF => {
                let value = self.read_r8_operand(opcode & 0b111);
                self.alu_cp_a(value);
            }
            0xCB => {
                let cb_opcode = self.next_u8();
                self.execute_cb(cb_opcode);
            }
            0xC0 => {
                self.ret_if(!self.read_flag(Flag::Zero));
            }
            0xC1 => {
                let value = self.pop_u16();
                self.write_u16(Register16::BC, value);
            }
            0xC2 => {
                let nn = self.next_u16();
                self.jump_if(!self.read_flag(Flag::Zero), nn);
            }
            0xC3 => {
                let nn = self.next_u16();
                self.jump(nn);
            }
            0xC4 => {
                let nn = self.next_u16();
                self.call_if(!self.read_flag(Flag::Zero), nn);
            }
            0xC5 => {
                self.push_u16(self.read_u16(Register16::BC));
            }
            0xC6 => {
                let value = self.next_u8();
                self.alu_add_a(value);
            }
            0xC7 => {
                self.rst(0x00);
            }
            0xC8 => {
                self.ret_if(self.read_flag(Flag::Zero));
            }
            0xC9 => {
                self.ret();
            }
            0xCE => {
                let value = self.next_u8();
                self.alu_adc_a(value);
            }
            0xCA => {
                let nn = self.next_u16();
                self.jump_if(self.read_flag(Flag::Zero), nn);
            }
            0xCC => {
                let nn = self.next_u16();
                self.call_if(self.read_flag(Flag::Zero), nn);
            }
            0xCD => {
                let nn = self.next_u16();
                self.call(nn);
            }
            0xCF => {
                self.rst(0x08);
            }
            0xD0 => {
                self.ret_if(!self.read_flag(Flag::Carry));
            }
            0xD1 => {
                let value = self.pop_u16();
                self.write_u16(Register16::DE, value);
            }
            0xD4 => {
                let nn = self.next_u16();
                self.call_if(!self.read_flag(Flag::Carry), nn);
            }
            0xD5 => {
                self.push_u16(self.read_u16(Register16::DE));
            }
            0xD6 => {
                let value = self.next_u8();
                self.alu_sub_a(value);
            }
            0xD7 => {
                self.rst(0x10);
            }
            0xD8 => {
                self.ret_if(self.read_flag(Flag::Carry));
            }
            0xD9 => {
                self.ret();
                self.interrupts_enabled = true;
            }
            0xD2 => {
                let nn = self.next_u16();
                self.jump_if(!self.read_flag(Flag::Carry), nn);
            }
            0xDC => {
                let nn = self.next_u16();
                self.call_if(self.read_flag(Flag::Carry), nn);
            }
            0xDE => {
                let value = self.next_u8();
                self.alu_sbc_a(value);
            }
            0xDF => {
                self.rst(0x18);
            }
            0xDA => {
                let nn = self.next_u16();
                self.jump_if(self.read_flag(Flag::Carry), nn);
            }
            0xE0 => {
                let address = 0xFF00 + u16::from(self.next_u8());
                self.write_u8_addr(address, self.read_u8(Register8::A));
            }
            0xE1 => {
                let value = self.pop_u16();
                self.write_u16(Register16::HL, value);
            }
            0xE2 => {
                let address = 0xFF00 + u16::from(self.read_u8(Register8::C));
                self.write_u8_addr(address, self.read_u8(Register8::A));
            }
            0xE5 => {
                self.push_u16(self.read_u16(Register16::HL));
            }
            0xE6 => {
                let value = self.next_u8();
                self.alu_and_a(value);
            }
            0xE7 => {
                self.rst(0x20);
            }
            0xE8 => {
                let offset = self.next_u8();
                self.sp = self.add_sp_e8(offset);
            }
            0xE9 => {
                self.jump(self.read_u16(Register16::HL));
            }
            0xEA => {
                let address = self.next_u16();
                self.write_u8_addr(address, self.read_u8(Register8::A));
            }
            0xEF => {
                self.rst(0x28);
            }
            0xEE => {
                let value = self.next_u8();
                self.alu_xor_a(value);
            }
            0xF0 => {
                let address = 0xFF00 + u16::from(self.next_u8());
                let value = self.read_u8_addr(address);
                self.write_u8(Register8::A, value);
            }
            0xF1 => {
                let value = self.pop_u16();
                self.write_u16(Register16::AF, value);
            }
            0xF2 => {
                let address = 0xFF00 + u16::from(self.read_u8(Register8::C));
                let value = self.read_u8_addr(address);
                self.write_u8(Register8::A, value);
            }
            0xF3 => {
                self.interrupts_enabled = false;
            }
            0xF5 => {
                self.push_u16(self.read_u16(Register16::AF));
            }
            0xF6 => {
                let value = self.next_u8();
                self.alu_or_a(value);
            }
            0xF7 => {
                self.rst(0x30);
            }
            0xF8 => {
                let offset = self.next_u8();
                let result = self.add_sp_e8(offset);
                self.write_u16(Register16::HL, result);
            }
            0xF9 => {
                self.sp = self.read_u16(Register16::HL);
            }
            0xFA => {
                let address = self.next_u16();
                let value = self.read_u8_addr(address);
                self.write_u8(Register8::A, value);
            }
            0xFB => {
                self.interrupts_enabled = true;
            }
            0xFE => {
                let value = self.next_u8();
                self.alu_cp_a(value);
            }
            0xFF => {
                self.rst(0x38);
            }
            code => panic!("Unknown opcode: {:#X} at {:#X}", code, self.pc),
        }
    }
}

impl Gameboy {
    pub fn read_u8_addr(&self, address: u16) -> u8 {
        self.mem[address as usize]
    }

    pub fn write_u8_addr(&mut self, address: u16, value: u8) {
        self.mem[address as usize] = value;
    }

    pub fn read_u16_addr(&self, address: u16) -> u16 {
        let lo = self.read_u8_addr(address);
        let hi = self.read_u8_addr(address.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    pub fn write_u16_addr(&mut self, address: u16, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.write_u8_addr(address, lo);
        self.write_u8_addr(address.wrapping_add(1), hi);
    }

    pub fn push_u16(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.sp = self.sp.wrapping_sub(1);
        self.write_u8_addr(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        self.write_u8_addr(self.sp, lo);
    }

    pub fn pop_u16(&mut self) -> u16 {
        let lo = self.read_u8_addr(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let hi = self.read_u8_addr(self.sp);
        self.sp = self.sp.wrapping_add(1);
        u16::from_le_bytes([lo, hi])
    }

    pub fn signed_e8(value: u8) -> i8 {
        value as i8
    }

    pub fn add_signed_e8(value: u16, offset: u8) -> u16 {
        value.wrapping_add_signed(i16::from(Self::signed_e8(offset)))
    }

    fn jump(&mut self, nn: u16) {
        self.pc = nn;
        self.cycles += 4;
    }

    fn jump_if(&mut self, condition: bool, nn: u16) {
        if condition {
            self.jump(nn);
        }
    }

    fn call(&mut self, nn: u16) {
        self.push_u16(self.pc);
        self.pc = nn;
        self.cycles += 12;
    }

    fn call_if(&mut self, condition: bool, nn: u16) {
        if condition {
            self.call(nn);
        }
    }

    fn ret(&mut self) {
        self.pc = self.pop_u16();
        self.cycles += 12;
    }

    fn ret_if(&mut self, condition: bool) {
        if condition {
            self.ret();
        }
    }

    fn rst(&mut self, vector: u16) {
        self.push_u16(self.pc);
        self.pc = vector;
        self.cycles += 12;
    }

    fn jump_relative(&mut self, offset: u8) {
        self.pc = Self::add_signed_e8(self.pc, offset);
        self.cycles += 4;
    }

    fn jump_relative_if(&mut self, condition: bool, offset: u8) {
        if condition {
            self.jump_relative(offset);
        }
    }

    fn next_u8(&mut self) -> u8 {
        let n = self.read_u8_addr(self.pc);
        self.pc = self.pc.wrapping_add(1);
        n
    }

    fn next_u16(&mut self) -> u16 {
        let n = self.read_u16_addr(self.pc);
        self.pc = self.pc.wrapping_add(2);
        n
    }

    fn read_r8_operand(&self, operand: u8) -> u8 {
        match operand {
            0 => self.read_u8(Register8::B),
            1 => self.read_u8(Register8::C),
            2 => self.read_u8(Register8::D),
            3 => self.read_u8(Register8::E),
            4 => self.read_u8(Register8::H),
            5 => self.read_u8(Register8::L),
            6 => self.read_u8_addr(self.read_u16(Register16::HL)),
            7 => self.read_u8(Register8::A),
            _ => unreachable!("r8 operand indexes are three bits"),
        }
    }

    fn write_r8_operand(&mut self, operand: u8, value: u8) {
        match operand {
            0 => self.write_u8(Register8::B, value),
            1 => self.write_u8(Register8::C, value),
            2 => self.write_u8(Register8::D, value),
            3 => self.write_u8(Register8::E, value),
            4 => self.write_u8(Register8::H, value),
            5 => self.write_u8(Register8::L, value),
            6 => self.write_u8_addr(self.read_u16(Register16::HL), value),
            7 => self.write_u8(Register8::A, value),
            _ => unreachable!("r8 operand indexes are three bits"),
        }
    }

    fn execute_cb(&mut self, opcode: u8) {
        let operand = opcode & 0b111;

        match opcode {
            0x00..=0x07 => self.rlc_r8_operand(operand),
            0x08..=0x0F => self.rrc_r8_operand(operand),
            0x10..=0x17 => self.rl_r8_operand(operand),
            0x18..=0x1F => self.rr_r8_operand(operand),
            0x20..=0x27 => self.sla_r8_operand(operand),
            0x28..=0x2F => self.sra_r8_operand(operand),
            0x30..=0x37 => self.swap_r8_operand(operand),
            0x38..=0x3F => self.srl_r8_operand(operand),
            0x40..=0x7F => self.bit_r8_operand((opcode >> 3) & 0b111, operand),
            0x80..=0xBF => self.res_r8_operand((opcode >> 3) & 0b111, operand),
            0xC0..=0xFF => self.set_r8_operand((opcode >> 3) & 0b111, operand),
        }
    }

    fn write_cb_shift_result(&mut self, operand: u8, result: u8, carry: bool) {
        self.write_r8_operand(operand, result);
        self.write_flag(Flag::Zero, result == 0);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, carry);
    }

    fn rlc_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_cb_shift_result(operand, value.rotate_left(1), value & 0x80 != 0);
    }

    fn rrc_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_cb_shift_result(operand, value.rotate_right(1), value & 0x01 != 0);
    }

    fn rl_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        let carry = u8::from(self.read_flag(Flag::Carry));
        self.write_cb_shift_result(operand, (value << 1) | carry, value & 0x80 != 0);
    }

    fn rr_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        let carry = u8::from(self.read_flag(Flag::Carry));
        self.write_cb_shift_result(operand, (value >> 1) | (carry << 7), value & 0x01 != 0);
    }

    fn sla_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_cb_shift_result(operand, value << 1, value & 0x80 != 0);
    }

    fn sra_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_cb_shift_result(operand, (value >> 1) | (value & 0x80), value & 0x01 != 0);
    }

    fn swap_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_cb_shift_result(operand, value.rotate_left(4), false);
    }

    fn srl_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_cb_shift_result(operand, value >> 1, value & 0x01 != 0);
    }

    fn bit_r8_operand(&mut self, bit: u8, operand: u8) {
        let value = self.read_r8_operand(operand);

        self.write_flag(Flag::Zero, value & (1 << bit) == 0);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, true);
    }

    fn res_r8_operand(&mut self, bit: u8, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_r8_operand(operand, value & !(1 << bit));
    }

    fn set_r8_operand(&mut self, bit: u8, operand: u8) {
        let value = self.read_r8_operand(operand);
        self.write_r8_operand(operand, value | (1 << bit));
    }

    fn inc_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        let result = value.wrapping_add(1);

        self.write_r8_operand(operand, result);
        self.write_flag(Flag::Zero, result == 0);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, (value & 0x0F) == 0x0F);
    }

    fn dec_r8_operand(&mut self, operand: u8) {
        let value = self.read_r8_operand(operand);
        let result = value.wrapping_sub(1);

        self.write_r8_operand(operand, result);
        self.write_flag(Flag::Zero, result == 0);
        self.write_flag(Flag::Subtraction, true);
        self.write_flag(Flag::HalfCarry, (value & 0x0F) == 0x00);
    }

    fn rlca(&mut self) {
        let a = self.read_u8(Register8::A);
        let result = a.rotate_left(1);

        self.write_u8(Register8::A, result);
        self.write_flag(Flag::Zero, false);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, a & 0x80 != 0);
    }

    fn rrca(&mut self) {
        let a = self.read_u8(Register8::A);
        let result = a.rotate_right(1);

        self.write_u8(Register8::A, result);
        self.write_flag(Flag::Zero, false);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, a & 0x01 != 0);
    }

    fn rla(&mut self) {
        let a = self.read_u8(Register8::A);
        let carry = u8::from(self.read_flag(Flag::Carry));
        let result = (a << 1) | carry;

        self.write_u8(Register8::A, result);
        self.write_flag(Flag::Zero, false);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, a & 0x80 != 0);
    }

    fn rra(&mut self) {
        let a = self.read_u8(Register8::A);
        let carry = u8::from(self.read_flag(Flag::Carry));
        let result = (a >> 1) | (carry << 7);

        self.write_u8(Register8::A, result);
        self.write_flag(Flag::Zero, false);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, a & 0x01 != 0);
    }

    fn daa(&mut self) {
        let mut a = self.read_u8(Register8::A);
        let mut adjust = 0;
        let mut carry = self.read_flag(Flag::Carry);

        if self.read_flag(Flag::Subtraction) {
            if self.read_flag(Flag::Carry) {
                adjust |= 0x60;
            }
            if self.read_flag(Flag::HalfCarry) {
                adjust |= 0x06;
            }
            a = a.wrapping_sub(adjust);
        } else {
            if self.read_flag(Flag::Carry) || a > 0x99 {
                adjust |= 0x60;
                carry = true;
            }
            if self.read_flag(Flag::HalfCarry) || (a & 0x0F) > 0x09 {
                adjust |= 0x06;
            }
            a = a.wrapping_add(adjust);
        }

        self.write_u8(Register8::A, a);
        self.write_flag(Flag::Zero, a == 0);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, carry);
    }

    fn cpl(&mut self) {
        let a = self.read_u8(Register8::A);

        self.write_u8(Register8::A, !a);
        self.write_flag(Flag::Subtraction, true);
        self.write_flag(Flag::HalfCarry, true);
    }

    fn scf(&mut self) {
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, true);
    }

    fn ccf(&mut self) {
        let carry = self.read_flag(Flag::Carry);

        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, false);
        self.write_flag(Flag::Carry, !carry);
    }

    fn add_hl(&mut self, value: u16) {
        let hl = self.read_u16(Register16::HL);
        let result = hl.wrapping_add(value);

        self.write_u16(Register16::HL, result);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(Flag::HalfCarry, (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.write_flag(Flag::Carry, u32::from(hl) + u32::from(value) > 0xFFFF);
    }

    fn add_sp_e8(&mut self, offset: u8) -> u16 {
        let sp = self.sp;
        let result = Self::add_signed_e8(sp, offset);

        self.write_flag(Flag::Zero, false);
        self.write_flag(Flag::Subtraction, false);
        self.write_flag(
            Flag::HalfCarry,
            (sp & 0x000F) + u16::from(offset & 0x0F) > 0x000F,
        );
        self.write_flag(Flag::Carry, (sp & 0x00FF) + u16::from(offset) > 0x00FF);

        result
    }
}
