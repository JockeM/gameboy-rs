#![allow(dead_code)]

use crate::registers::*;

pub struct Gameboy {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16,

    pub mem: [u8; 0xFFFF],
    pub cycles: u64,
}

impl Gameboy {
    pub fn load(rom_bytes: &[u8]) -> Self {
        let mut mem = [0; 0xFFFF];
        mem[..rom_bytes.len()].copy_from_slice(rom_bytes);

        Self {
            af: 0,
            bc: 0,
            de: 0,
            hl: 0,
            sp: 0,
            pc: 0x100,
            mem,
            cycles: 0,
        }
    }

    pub fn run(&mut self) {
        // loop that executes and uses mem
        loop {
            self.execute();
        }
    }

    pub fn execute(&mut self) {
        let slice = self.mem[self.pc as usize];
        let next = match slice {
            0x00 => self.pc + 1,
            0x01 => {
                let nn = self.next_u16();
                self.write_u16(Register16::BC, nn);
                self.pc + 3
            }
            0x02 => {
                let bc = self.read_u16(Register16::BC);
                self.mem[bc as usize] = self.read_u8(Register8::A);
                self.pc + 3
            }
            0x03 => {
                let bc = self.read_u16(Register16::BC);
                self.write_u16(Register16::BC, bc.wrapping_add_signed(1));
                self.pc + 2
            }
            0x04 => {
                let b = self.read_u8(Register8::B);
                self.write_u8(Register8::B, b.wrapping_add_signed(1));
                self.pc + 1
            }
            0x05 => {
                let b = self.read_u8(Register8::B);
                self.write_u8(Register8::B, b.wrapping_add_signed(-1));
                self.pc + 1
            }
            0x06 => {
                let n = self.next_u8();
                self.write_u8(Register8::B, n);
                self.pc + 2
            }
            0x07 => {
                let b = self.read_u8(Register8::B);
                self.write_u8(Register8::B, b >> 1);
                self.pc + 2
            }
            0xC3 => {
                let nn = self.next_u16();
                self.jump(nn);
                self.pc + 3
            }
            code => panic!("Unknown opcode: {:#X} at {:#X}", code, self.pc),
        };

        self.pc = next;
    }
}

impl Gameboy {
    fn jump(&mut self, nn: u16) {
        self.pc = nn;
        self.cycles += 4;
    }

    fn next_u8(&mut self) -> u8 {
        let n = self.mem[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        n
    }

    fn next_u16(&mut self) -> u16 {
        let n0 = self.mem[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        let n1 = self.mem[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        u16::from_ne_bytes([n0, n1])
    }
    // Load the 16-bit immediate operand a16 into the program counter PC if the Z flag is 0. If the Z flag is 0, then the subsequent instruction starts at address a16. If not, the contents of PC are incremented, and the next instruction following the current JP instruction is executed (as usual).
    // The second byte of the object code (immediately following the opcode) corresponds to the lower-order byte of a16 (bits 0-7), and the third byte of the object code corresponds to the higher-order byte (bits 8-15).
    fn jump_if_zero(&mut self, nn: u16) {
        if self.read_flag(Flag::Zero) {
            self.pc = nn;
            self.cycles += 4;
        } else {
            self.pc += 3;
        }
    }
}
