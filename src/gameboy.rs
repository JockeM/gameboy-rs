#![allow(dead_code)]

use crate::ppu::Ppu;
#[cfg(feature = "window")]
use crate::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::registers::*;

#[cfg(feature = "window")]
use minifb::{Key, Scale, Window, WindowOptions};

use std::fs;
use std::io::{self, Write};
use std::path::Path;
#[cfg(feature = "window")]
use std::time::Duration;

#[cfg(feature = "window")]
const CPU_CLOCK_HZ: u64 = 4_194_304;
const CYCLES_PER_FRAME: u64 = 70_224;
#[cfg(feature = "window")]
const NANOS_PER_FRAME: u64 = CYCLES_PER_FRAME * 1_000_000_000 / CPU_CLOCK_HZ;
const INTERRUPT_ENABLE_ADDR: usize = 0xFFFF;
const INTERRUPT_FLAG_ADDR: usize = 0xFF0F;
const KEY1_ADDR: usize = 0xFF4D;
const DIV_ADDR: usize = 0xFF04;
const TIMA_ADDR: usize = 0xFF05;
const TMA_ADDR: usize = 0xFF06;
const TAC_ADDR: usize = 0xFF07;

fn initialize_io_registers(mem: &mut [u8; 0x10000]) {
    mem[0xFF00] = 0xCF;
    mem[0xFF05] = 0x00;
    mem[0xFF06] = 0x00;
    mem[0xFF07] = 0x00;
    mem[0xFF10] = 0x80;
    mem[0xFF11] = 0xBF;
    mem[0xFF12] = 0xF3;
    mem[0xFF14] = 0xBF;
    mem[0xFF16] = 0x3F;
    mem[0xFF17] = 0x00;
    mem[0xFF19] = 0xBF;
    mem[0xFF1A] = 0x7F;
    mem[0xFF1B] = 0xFF;
    mem[0xFF1C] = 0x9F;
    mem[0xFF1E] = 0xBF;
    mem[0xFF20] = 0xFF;
    mem[0xFF21] = 0x00;
    mem[0xFF22] = 0x00;
    mem[0xFF23] = 0xBF;
    mem[0xFF24] = 0x77;
    mem[0xFF25] = 0xF3;
    mem[0xFF26] = 0xF1;
    mem[0xFF40] = 0x91;
    mem[0xFF42] = 0x00;
    mem[0xFF43] = 0x00;
    mem[0xFF45] = 0x00;
    mem[0xFF47] = 0xFC;
    mem[0xFF48] = 0xFF;
    mem[0xFF49] = 0xFF;
    mem[0xFF4A] = 0x00;
    mem[0xFF4B] = 0x00;
    mem[0xFFFF] = 0x00;
}

struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mapper: Mapper,
    rom_bank_count: usize,
    ram_bank_count: usize,
}

enum Mapper {
    NoMbc,
    Mbc1 {
        ram_enabled: bool,
        rom_bank_low5: u8,
        bank_high2: u8,
        banking_mode: u8,
    },
}

impl Cartridge {
    fn new(rom_bytes: &[u8]) -> Self {
        let rom = rom_bytes.to_vec();
        let rom_bank_count = (rom.len().max(0x4000) + 0x3FFF) / 0x4000;
        let ram_size = rom_bytes
            .get(0x149)
            .copied()
            .map(cartridge_ram_size)
            .unwrap_or(0);
        let ram_bank_count = if ram_size == 0 {
            0
        } else {
            (ram_size + 0x1FFF) / 0x2000
        };
        let mapper = match rom_bytes.get(0x147).copied().unwrap_or(0x00) {
            0x00 => Mapper::NoMbc,
            0x01..=0x03 => Mapper::Mbc1 {
                ram_enabled: false,
                rom_bank_low5: 1,
                bank_high2: 0,
                banking_mode: 0,
            },
            cartridge_type => panic!("Unsupported cartridge type: {cartridge_type:#04X}"),
        };

        Self {
            rom,
            ram: vec![0; ram_size],
            mapper,
            rom_bank_count,
            ram_bank_count,
        }
    }

    fn read_rom(&self, address: u16) -> u8 {
        let address = usize::from(address);
        let bank = if address < 0x4000 {
            self.lower_rom_bank()
        } else {
            self.upper_rom_bank()
        };
        let offset = bank * 0x4000 + (address & 0x3FFF);

        self.rom.get(offset).copied().unwrap_or(0xFF)
    }

    fn write_rom(&mut self, address: u16, value: u8) {
        match &mut self.mapper {
            Mapper::NoMbc => {}
            Mapper::Mbc1 {
                ram_enabled,
                rom_bank_low5,
                bank_high2,
                banking_mode,
            } => match address {
                0x0000..=0x1FFF => *ram_enabled = value & 0x0F == 0x0A,
                0x2000..=0x3FFF => {
                    let bank = value & 0x1F;
                    *rom_bank_low5 = if bank == 0 { 1 } else { bank };
                }
                0x4000..=0x5FFF => *bank_high2 = value & 0x03,
                0x6000..=0x7FFF => *banking_mode = value & 0x01,
                _ => unreachable!("invalid ROM write address {address:#06X}"),
            },
        }
    }

    fn read_ram(&self, address: u16) -> u8 {
        self.ram_index(address)
            .and_then(|index| self.ram.get(index).copied())
            .unwrap_or(0xFF)
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        if let Some(index) = self.ram_index(address) {
            self.ram[index] = value;
        }
    }

    fn is_present(&self) -> bool {
        !self.rom.is_empty()
    }

    fn sync_visible_memory(&self, mem: &mut [u8; 0x10000]) {
        self.copy_rom_bank_into(mem, 0x0000, self.lower_rom_bank());
        self.copy_rom_bank_into(mem, 0x4000, self.upper_rom_bank());
        self.sync_external_ram(mem);
    }

    fn copy_rom_bank_into(&self, mem: &mut [u8; 0x10000], start: usize, bank: usize) {
        let window = &mut mem[start..start + 0x4000];
        window.fill(0xFF);

        let offset = bank * 0x4000;
        let available = self.rom.len().saturating_sub(offset).min(0x4000);
        if available > 0 {
            window[..available].copy_from_slice(&self.rom[offset..offset + available]);
        }
    }

    fn sync_external_ram(&self, mem: &mut [u8; 0x10000]) {
        let window = &mut mem[0xA000..0xC000];
        window.fill(0xFF);

        if self.ram.is_empty() {
            return;
        }

        let bank = self.current_ram_bank();
        let offset = bank * 0x2000;
        let available = self.ram.len().saturating_sub(offset).min(0x2000);
        if available > 0 {
            window[..available].copy_from_slice(&self.ram[offset..offset + available]);
        }
    }

    fn lower_rom_bank(&self) -> usize {
        match self.mapper {
            Mapper::NoMbc => 0,
            Mapper::Mbc1 {
                bank_high2,
                banking_mode,
                ..
            } if banking_mode != 0 => (usize::from(bank_high2) << 5) % self.rom_bank_count,
            Mapper::Mbc1 { .. } => 0,
        }
    }

    fn upper_rom_bank(&self) -> usize {
        match self.mapper {
            Mapper::NoMbc => {
                if self.rom_bank_count > 1 {
                    1
                } else {
                    0
                }
            }
            Mapper::Mbc1 {
                rom_bank_low5,
                bank_high2,
                ..
            } => {
                let bank = ((usize::from(bank_high2) << 5) | usize::from(rom_bank_low5))
                    % self.rom_bank_count;
                if bank == 0 && self.rom_bank_count > 1 {
                    1
                } else {
                    bank
                }
            }
        }
    }

    fn current_ram_bank(&self) -> usize {
        if self.ram_bank_count == 0 {
            return 0;
        }

        match self.mapper {
            Mapper::Mbc1 {
                bank_high2,
                banking_mode,
                ..
            } if banking_mode != 0 => usize::from(bank_high2) % self.ram_bank_count,
            _ => 0,
        }
    }

    fn ram_index(&self, address: u16) -> Option<usize> {
        if self.ram.is_empty() || !(0xA000..=0xBFFF).contains(&address) {
            return None;
        }

        match self.mapper {
            Mapper::Mbc1 { ram_enabled, .. } if !ram_enabled => None,
            _ => {
                let offset = usize::from(address - 0xA000);
                let index = self.current_ram_bank() * 0x2000 + offset;
                (index < self.ram.len()).then_some(index)
            }
        }
    }
}

fn cartridge_ram_size(code: u8) -> usize {
    match code {
        0x00 => 0,
        0x01 => 0x0800,
        0x02 => 0x2000,
        0x03 => 0x8000,
        0x04 => 0x20000,
        0x05 => 0x10000,
        _ => 0,
    }
}

pub struct Gameboy {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16,

    pub mem: [u8; 0x10000],
    cartridge: Cartridge,
    pub ppu: Ppu,
    pub cycles: u64,
    pub frames: u64,
    pub halted: bool,
    pub stopped: bool,
    pub interrupts_enabled: bool,
    pub serial_output: Vec<u8>,
    timer_counter: u16,
    tima_reload_delay: u8,
    joypad_buttons: u8,
    joypad_directions: u8,
    ppu_pending: u64,
}

impl Gameboy {
    pub fn load_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let rom_bytes = fs::read(path)?;
        Ok(Self::load(&rom_bytes))
    }

    pub fn load(rom_bytes: &[u8]) -> Self {
        Self::load_inner(rom_bytes, false)
    }

    pub fn load_headless(rom_bytes: &[u8]) -> Self {
        Self::load_inner(rom_bytes, true)
    }

    fn load_inner(rom_bytes: &[u8], headless: bool) -> Self {
        let mut mem = [0; 0x10000];
        initialize_io_registers(&mut mem);
        let cartridge = Cartridge::new(rom_bytes);
        if cartridge.is_present() {
            cartridge.sync_visible_memory(&mut mem);
        }

        let mut ppu = if headless { Ppu::new_headless() } else { Ppu::new() };
        ppu.sync_registers(&mut mem);

        Self {
            af: 0x01B0,
            bc: 0x0013,
            de: 0x00D8,
            hl: 0x014D,
            sp: 0xFFFE,
            pc: 0x100,
            mem,
            cartridge,
            ppu,
            cycles: 0,
            frames: 0,
            halted: false,
            stopped: false,
            interrupts_enabled: false,
            serial_output: Vec::new(),
            timer_counter: 0,
            tima_reload_delay: 0,
            joypad_buttons: 0x0F,
            joypad_directions: 0x0F,
            ppu_pending: 0,
        }
    }

    #[cfg(feature = "window")]
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
        window.limit_update_rate(Some(Duration::from_nanos(NANOS_PER_FRAME)));

        while window.is_open() && !window.is_key_down(Key::Escape) {
            self.update_joypad(&window);
            self.run_frame();
            window.update_with_buffer(&self.ppu.framebuffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;
        }

        Ok(())
    }

    pub fn run_frame(&mut self) {
        let target_cycles = self.cycles + CYCLES_PER_FRAME;

        while self.cycles < target_cycles && !self.stopped {
            let previous_cycles = self.cycles;

            // Skip the full interrupt path unless interrupts are enabled or
            // the CPU is halted — avoids reading two memory locations every instruction.
            if (self.interrupts_enabled || self.halted) && self.service_interrupt() {
            } else if self.halted {
                self.cycles += 4;
            } else {
                self.execute();
                self.cycles += 4;
            }

            let elapsed_cycles = self.cycles - previous_cycles;
            self.advance_timer(elapsed_cycles);

            // Accumulate PPU cycles and only step when we reach a mode boundary.
            // Minimum mode duration is 80 cycles (OAM scan), so most instructions
            // (~4 cycles each) are skipped entirely.
            self.ppu_pending += elapsed_cycles;
            if self.ppu_pending >= u64::from(self.ppu.cycles_until_mode_end()) {
                self.ppu.step(&mut self.mem, self.ppu_pending);
                self.ppu_pending = 0;
            }
        }

        // Flush any remaining accumulated PPU cycles.
        if self.ppu_pending > 0 {
            self.ppu.step(&mut self.mem, self.ppu_pending);
            self.ppu_pending = 0;
        }

        self.frames += 1;
    }

    pub fn execute(&mut self) {
        let opcode = self.next_u8();

        match opcode {
            0x00 => {}
            0x01 => {
                let nn = self.next_u16();
                self.write_u16(Register16::BC, nn);
                self.cycles += 8;
            }
            0x02 => {
                let bc = self.read_u16(Register16::BC);
                self.write_u8_addr(bc, self.read_u8(Register8::A));
                self.cycles += 4;
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
                self.cycles += 4;
            }
            0x07 => {
                self.rlca();
            }
            0x08 => {
                let address = self.next_u16();
                self.write_u16_addr(address, self.sp);
                self.cycles += 16;
            }
            0x09 => {
                self.add_hl(self.read_u16(Register16::BC));
                self.cycles += 4;
            }
            0x0A => {
                let bc = self.read_u16(Register16::BC);
                let value = self.read_u8_addr(bc);
                self.write_u8(Register8::A, value);
                self.cycles += 4;
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
                self.cycles += 4;
            }
            0x0F => {
                self.rrca();
            }
            0x10 => {
                self.next_u8();
                if self.mem[KEY1_ADDR] & 0x01 != 0 {
                    self.mem[KEY1_ADDR] ^= 0x80;
                    self.mem[KEY1_ADDR] &= 0x80;
                } else {
                    self.stopped = true;
                }
            }
            0x11 => {
                let nn = self.next_u16();
                self.write_u16(Register16::DE, nn);
                self.cycles += 8;
            }
            0x12 => {
                let de = self.read_u16(Register16::DE);
                self.write_u8_addr(de, self.read_u8(Register8::A));
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.cycles += 4;
            }
            0x1A => {
                let de = self.read_u16(Register16::DE);
                let value = self.read_u8_addr(de);
                self.write_u8(Register8::A, value);
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.cycles += 8;
            }
            0x22 => {
                let hl = self.read_u16(Register16::HL);
                self.write_u8_addr(hl, self.read_u8(Register8::A));
                self.write_u16(Register16::HL, hl.wrapping_add(1));
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.cycles += 4;
            }
            0x2A => {
                let hl = self.read_u16(Register16::HL);
                let value = self.read_u8_addr(hl);
                self.write_u8(Register8::A, value);
                self.write_u16(Register16::HL, hl.wrapping_add(1));
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.cycles += 8;
            }
            0x32 => {
                let hl = self.read_u16(Register16::HL);
                self.write_u8_addr(hl, self.read_u8(Register8::A));
                self.write_u16(Register16::HL, hl.wrapping_sub(1));
                self.cycles += 4;
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
            }
            0x34 => {
                self.inc_r8_operand(6);
                self.cycles += 8;
            }
            0x35 => {
                self.dec_r8_operand(6);
                self.cycles += 8;
            }
            0x36 => {
                let n = self.next_u8();
                self.write_u8_addr(self.read_u16(Register16::HL), n);
                self.cycles += 8;
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
                self.cycles += 4;
            }
            0x3A => {
                let hl = self.read_u16(Register16::HL);
                let value = self.read_u8_addr(hl);
                self.write_u8(Register8::A, value);
                self.write_u16(Register16::HL, hl.wrapping_sub(1));
                self.cycles += 4;
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
                self.cycles += 4;
            }
            0x3F => {
                self.ccf();
            }
            0x40..=0x75 | 0x77..=0x7F => {
                let destination = (opcode >> 3) & 0b111;
                let source = opcode & 0b111;
                let value = self.read_r8_operand(source);
                self.write_r8_operand(destination, value);
                if source == 6 || destination == 6 {
                    self.cycles += 4;
                }
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
                self.cycles += 8;
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
                self.cycles += 12;
            }
            0xC6 => {
                let value = self.next_u8();
                self.alu_add_a(value);
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.cycles += 8;
            }
            0xD4 => {
                let nn = self.next_u16();
                self.call_if(!self.read_flag(Flag::Carry), nn);
            }
            0xD5 => {
                self.push_u16(self.read_u16(Register16::DE));
                self.cycles += 12;
            }
            0xD6 => {
                let value = self.next_u8();
                self.alu_sub_a(value);
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.cycles += 8;
            }
            0xE1 => {
                let value = self.pop_u16();
                self.write_u16(Register16::HL, value);
                self.cycles += 8;
            }
            0xE2 => {
                let address = 0xFF00 + u16::from(self.read_u8(Register8::C));
                self.write_u8_addr(address, self.read_u8(Register8::A));
                self.cycles += 4;
            }
            0xE5 => {
                self.push_u16(self.read_u16(Register16::HL));
                self.cycles += 12;
            }
            0xE6 => {
                let value = self.next_u8();
                self.alu_and_a(value);
                self.cycles += 4;
            }
            0xE7 => {
                self.rst(0x20);
            }
            0xE8 => {
                let offset = self.next_u8();
                self.sp = self.add_sp_e8(offset);
                self.cycles += 12;
            }
            0xE9 => {
                self.pc = self.read_u16(Register16::HL);
            }
            0xEA => {
                let address = self.next_u16();
                self.write_u8_addr(address, self.read_u8(Register8::A));
                self.cycles += 12;
            }
            0xEF => {
                self.rst(0x28);
            }
            0xEE => {
                let value = self.next_u8();
                self.alu_xor_a(value);
                self.cycles += 4;
            }
            0xF0 => {
                let address = 0xFF00 + u16::from(self.next_u8());
                let value = self.read_u8_addr(address);
                self.write_u8(Register8::A, value);
                self.cycles += 8;
            }
            0xF1 => {
                let value = self.pop_u16();
                self.write_u16(Register16::AF, value);
                self.cycles += 8;
            }
            0xF2 => {
                let address = 0xFF00 + u16::from(self.read_u8(Register8::C));
                let value = self.read_u8_addr(address);
                self.write_u8(Register8::A, value);
                self.cycles += 4;
            }
            0xF3 => {
                self.interrupts_enabled = false;
            }
            0xF5 => {
                self.push_u16(self.read_u16(Register16::AF));
                self.cycles += 12;
            }
            0xF6 => {
                let value = self.next_u8();
                self.alu_or_a(value);
                self.cycles += 4;
            }
            0xF7 => {
                self.rst(0x30);
            }
            0xF8 => {
                let offset = self.next_u8();
                let result = self.add_sp_e8(offset);
                self.write_u16(Register16::HL, result);
                self.cycles += 8;
            }
            0xF9 => {
                self.sp = self.read_u16(Register16::HL);
                self.cycles += 4;
            }
            0xFA => {
                let address = self.next_u16();
                let value = self.read_u8_addr(address);
                self.write_u8(Register8::A, value);
                self.cycles += 12;
            }
            0xFB => {
                self.interrupts_enabled = true;
            }
            0xFE => {
                let value = self.next_u8();
                self.alu_cp_a(value);
                self.cycles += 4;
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
        match address {
            0xA000..=0xBFFF if self.cartridge.is_present() => self.cartridge.read_ram(address),
            0xFF00 => self.read_joypad(),
            _ => self.mem[address as usize],
        }
    }

    pub fn write_u8_addr(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF if self.cartridge.is_present() => {
                self.cartridge.write_rom(address, value);
                self.cartridge.sync_visible_memory(&mut self.mem);
            }
            0xA000..=0xBFFF if self.cartridge.is_present() => {
                self.cartridge.write_ram(address, value);
                self.cartridge.sync_external_ram(&mut self.mem);
            }
            0xC000..=0xDDFF => {
                self.mem[address as usize] = value;
                self.mem[usize::from(address + 0x2000)] = value;
            }
            0xE000..=0xFDFF => {
                self.mem[address as usize] = value;
                self.mem[usize::from(address - 0x2000)] = value;
            }
            0xFEA0..=0xFEFF => {}
            0xFF00 => {
                self.write_joypad_select(value);
            }
            0xFF01 => {
                self.mem[0xFF01] = value;
            }
            0xFF02 => {
                self.mem[0xFF02] = value;
                if value == 0x81 {
                    self.emit_serial_byte(self.mem[0xFF01]);
                    self.mem[0xFF02] = 0x01;
                }
            }
            0xFF04 => {
                self.reset_divider();
            }
            0xFF05 => {
                self.tima_reload_delay = 0;
                self.mem[TIMA_ADDR] = value;
            }
            0xFF06 => {
                self.mem[TMA_ADDR] = value;
            }
            0xFF40 => {
                self.ppu.write_lcdc(&mut self.mem, value);
            }
            0xFF41 => {
                self.ppu.write_stat(&mut self.mem, value);
            }
            0xFF44 => {
                self.ppu.reset_ly(&mut self.mem);
            }
            0xFF07 => {
                self.write_tac(value);
            }
            0xFF46 => {
                self.perform_oam_dma(value);
            }
            0xFF4D => {
                self.mem[KEY1_ADDR] = (self.mem[KEY1_ADDR] & 0x80) | (value & 0x01);
            }
            _ => self.mem[address as usize] = value,
        }
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
        self.cycles += 12;
    }

    fn jump_if(&mut self, condition: bool, nn: u16) {
        if condition {
            self.jump(nn);
        } else {
            self.cycles += 8;
        }
    }

    fn call(&mut self, nn: u16) {
        self.push_u16(self.pc);
        self.pc = nn;
        self.cycles += 20;
    }

    fn call_if(&mut self, condition: bool, nn: u16) {
        if condition {
            self.call(nn);
        } else {
            self.cycles += 8;
        }
    }

    fn ret(&mut self) {
        self.pc = self.pop_u16();
        self.cycles += 12;
    }

    fn ret_if(&mut self, condition: bool) {
        if condition {
            self.cycles += 4;
            self.ret();
        } else {
            self.cycles += 4;
        }
    }

    fn rst(&mut self, vector: u16) {
        self.push_u16(self.pc);
        self.pc = vector;
        self.cycles += 12;
    }

    fn jump_relative(&mut self, offset: u8) {
        self.pc = Self::add_signed_e8(self.pc, offset);
        self.cycles += 8;
    }

    fn jump_relative_if(&mut self, condition: bool, offset: u8) {
        if condition {
            self.jump_relative(offset);
        } else {
            self.cycles += 4;
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

    #[cfg(feature = "window")]
    fn update_joypad(&mut self, window: &Window) {
        let mut directions = 0x0F;
        let mut buttons = 0x0F;

        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            directions &= !0x01;
        }
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            directions &= !0x02;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            directions &= !0x04;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            directions &= !0x08;
        }
        if window.is_key_down(Key::Z) || window.is_key_down(Key::J) {
            buttons &= !0x01;
        }
        if window.is_key_down(Key::X) || window.is_key_down(Key::K) {
            buttons &= !0x02;
        }
        if window.is_key_down(Key::Backspace) || window.is_key_down(Key::RightShift) {
            buttons &= !0x04;
        }
        if window.is_key_down(Key::Enter) || window.is_key_down(Key::Space) {
            buttons &= !0x08;
        }

        self.set_joypad_state(buttons, directions);
    }

    fn read_joypad(&self) -> u8 {
        let select = self.mem[0xFF00] & 0x30;
        self.joypad_output(select)
    }

    fn joypad_output(&self, select: u8) -> u8 {
        let mut low = 0x0F;

        if select & 0x20 == 0 {
            low &= self.joypad_buttons;
        }
        if select & 0x10 == 0 {
            low &= self.joypad_directions;
        }

        0xC0 | select | low
    }

    pub fn set_joypad_state(&mut self, buttons: u8, directions: u8) {
        let previous = self.read_joypad();
        self.joypad_buttons = buttons;
        self.joypad_directions = directions;
        self.request_joypad_interrupt(previous, self.read_joypad());
    }

    fn write_joypad_select(&mut self, value: u8) {
        let previous = self.read_joypad();
        self.mem[0xFF00] = (self.mem[0xFF00] & 0xCF) | (value & 0x30);
        self.request_joypad_interrupt(previous, self.read_joypad());
    }

    fn request_joypad_interrupt(&mut self, previous: u8, current: u8) {
        if (previous & !current) & 0x0F != 0 {
            self.mem[INTERRUPT_FLAG_ADDR] |= 0x10;
        }
    }

    fn perform_oam_dma(&mut self, value: u8) {
        self.mem[0xFF46] = value;

        let source_base = u16::from(value) << 8;
        for offset in 0..0xA0u16 {
            let byte = self.read_u8_addr(source_base.wrapping_add(offset));
            self.mem[usize::from(0xFE00 + offset)] = byte;
        }
    }

    fn pending_interrupts(&self) -> u8 {
        self.mem[INTERRUPT_ENABLE_ADDR] & self.mem[INTERRUPT_FLAG_ADDR] & 0x1F
    }

    fn emit_serial_byte(&mut self, value: u8) {
        self.serial_output.push(value);
        let _ = io::stdout().write_all(&[value]);
        let _ = io::stdout().flush();
    }

    fn advance_timer(&mut self, elapsed_cycles: u64) {
        // Process any pending TIMA reload tick-by-tick (max 4 iterations).
        // The reload must fire at the right cycle, so we can't batch it.
        let reload_ticks = elapsed_cycles.min(self.tima_reload_delay as u64);
        for _ in 0..reload_ticks {
            self.advance_tima_reload();
            let old_signal = self.timer_signal();
            self.timer_counter = self.timer_counter.wrapping_add(1);
            self.mem[DIV_ADDR] = (self.timer_counter >> 8) as u8;
            self.tick_tima_on_falling_edge(old_signal);
        }

        let remaining = elapsed_cycles - reload_ticks;
        if remaining == 0 {
            return;
        }

        // Batch the remaining cycles: advance counter in one step and count
        // how many falling edges of the TIMA bit occurred in (old, old+remaining].
        // advance_tima_reload is a no-op here (delay is now 0).
        let timer_enabled = self.mem[TAC_ADDR] & 0x04 != 0;
        let old_counter = self.timer_counter;
        self.timer_counter = self.timer_counter.wrapping_add(remaining as u16);
        self.mem[DIV_ADDR] = (self.timer_counter >> 8) as u8;

        if timer_enabled {
            // A falling edge on `bit` occurs at every multiple of `2 * bit`.
            let bit = u64::from(self.tima_counter_bit());
            let period = bit * 2;
            let start = u64::from(old_counter);
            let end = start + remaining;
            let falling_edges = end / period - start / period;
            for _ in 0..falling_edges {
                self.increment_tima();
            }
        }
    }

    fn reset_divider(&mut self) {
        let old_signal = self.timer_signal();

        self.timer_counter = 0;
        self.mem[DIV_ADDR] = 0;
        self.tick_tima_on_falling_edge(old_signal);
    }

    fn write_tac(&mut self, value: u8) {
        let old_signal = self.timer_signal();

        self.mem[TAC_ADDR] = value & 0x07;
        self.tick_tima_on_falling_edge(old_signal);
    }

    fn timer_signal(&self) -> bool {
        self.mem[TAC_ADDR] & 0x04 != 0 && self.timer_counter & self.tima_counter_bit() != 0
    }

    fn tima_counter_bit(&self) -> u16 {
        match self.mem[TAC_ADDR] & 0x03 {
            0x00 => 1 << 9,
            0x01 => 1 << 3,
            0x02 => 1 << 5,
            0x03 => 1 << 7,
            _ => unreachable!("TAC frequency is masked to two bits"),
        }
    }

    fn tick_tima_on_falling_edge(&mut self, old_signal: bool) {
        if old_signal && !self.timer_signal() {
            self.increment_tima();
        }
    }

    fn increment_tima(&mut self) {
        let (value, overflowed) = self.mem[TIMA_ADDR].overflowing_add(1);

        if overflowed {
            self.mem[TIMA_ADDR] = 0;
            self.tima_reload_delay = 4;
        } else {
            self.mem[TIMA_ADDR] = value;
        }
    }

    fn advance_tima_reload(&mut self) {
        if self.tima_reload_delay == 0 {
            return;
        }

        self.tima_reload_delay -= 1;

        if self.tima_reload_delay == 0 {
            self.mem[TIMA_ADDR] = self.mem[TMA_ADDR];
            self.mem[INTERRUPT_FLAG_ADDR] |= 0x04;
        }
    }

    fn advance_ppu(&mut self, elapsed_cycles: u64) {
        self.ppu.step(&mut self.mem, elapsed_cycles);
    }

    fn service_interrupt(&mut self) -> bool {
        let pending = self.pending_interrupts();

        if pending == 0 {
            return false;
        }

        self.halted = false;

        if !self.interrupts_enabled {
            return false;
        }

        let interrupt = pending.trailing_zeros() as u8;
        let vector = match interrupt {
            0 => 0x40,
            1 => 0x48,
            2 => 0x50,
            3 => 0x58,
            4 => 0x60,
            _ => unreachable!("pending interrupts are masked to five bits"),
        };

        self.interrupts_enabled = false;
        self.mem[INTERRUPT_FLAG_ADDR] &= !(1 << interrupt);
        self.push_u16(self.pc);
        self.pc = vector;
        self.cycles += 20;

        true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_ppu_sets_ly_and_requests_vblank() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.advance_ppu(
            u64::from(crate::ppu::VISIBLE_SCANLINES) * u64::from(crate::ppu::CYCLES_PER_SCANLINE),
        );

        assert_eq!(gameboy.mem[0xFF44], crate::ppu::VISIBLE_SCANLINES);
        assert_eq!(gameboy.mem[INTERRUPT_FLAG_ADDR] & 0x01, 0x01);
    }

    #[test]
    fn advance_ppu_wraps_ly_at_frame_boundary() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.advance_ppu(
            u64::from(crate::ppu::TOTAL_SCANLINES) * u64::from(crate::ppu::CYCLES_PER_SCANLINE),
        );

        assert_eq!(gameboy.mem[0xFF44], 0);
        assert_eq!(gameboy.mem[0xFF41] & 0x03, 0x02);
    }

    #[test]
    fn advance_timer_updates_div_from_elapsed_cycles() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);

        gameboy.advance_timer(255);
        assert_eq!(gameboy.mem[DIV_ADDR], 0);

        gameboy.advance_timer(1);
        assert_eq!(gameboy.mem[DIV_ADDR], 1);
    }

    #[test]
    fn advance_timer_increments_tima_at_tac_frequency() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.write_u8_addr(TAC_ADDR as u16, 0x05);

        gameboy.advance_timer(15);
        assert_eq!(gameboy.mem[TIMA_ADDR], 0);

        gameboy.advance_timer(1);
        assert_eq!(gameboy.mem[TIMA_ADDR], 1);
    }

    #[test]
    fn advance_timer_reload_tima_and_requests_timer_interrupt_on_overflow() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.mem[TIMA_ADDR] = 0xFF;
        gameboy.mem[TMA_ADDR] = 0x42;
        gameboy.write_u8_addr(TAC_ADDR as u16, 0x05);

        gameboy.advance_timer(16);
        assert_eq!(gameboy.mem[TIMA_ADDR], 0x00);
        assert_eq!(gameboy.mem[INTERRUPT_FLAG_ADDR] & 0x04, 0x00);

        gameboy.advance_timer(4);

        assert_eq!(gameboy.mem[TIMA_ADDR], 0x42);
        assert_eq!(gameboy.mem[INTERRUPT_FLAG_ADDR] & 0x04, 0x04);
    }

    #[test]
    fn disabled_timer_does_not_increment_tima() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.write_u8_addr(TAC_ADDR as u16, 0x01);

        gameboy.advance_timer(64);

        assert_eq!(gameboy.mem[TIMA_ADDR], 0);
    }

    #[test]
    fn resetting_divider_can_increment_tima_on_falling_edge() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.write_u8_addr(TAC_ADDR as u16, 0x05);
        gameboy.advance_timer(8);

        gameboy.write_u8_addr(DIV_ADDR as u16, 0x00);

        assert_eq!(gameboy.mem[DIV_ADDR], 0);
        assert_eq!(gameboy.mem[TIMA_ADDR], 1);
    }

    #[test]
    fn read_joypad_uses_selected_button_group() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.joypad_buttons = 0b1110;
        gameboy.joypad_directions = 0b1101;

        gameboy.write_u8_addr(0xFF00, 0x20);
        assert_eq!(gameboy.read_u8_addr(0xFF00) & 0x0F, 0b1101);

        gameboy.write_u8_addr(0xFF00, 0x10);
        assert_eq!(gameboy.read_u8_addr(0xFF00) & 0x0F, 0b1110);

        gameboy.write_u8_addr(0xFF00, 0x00);
        assert_eq!(gameboy.read_u8_addr(0xFF00) & 0x0F, 0b1100);
    }

    #[test]
    fn pressed_joypad_button_requests_interrupt() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.write_u8_addr(0xFF00, 0x10);

        gameboy.set_joypad_state(0b1110, 0x0F);

        assert_eq!(gameboy.mem[INTERRUPT_FLAG_ADDR] & 0x10, 0x10);
    }

    #[test]
    fn selecting_pressed_joypad_group_requests_interrupt() {
        let mut gameboy = Gameboy::load(&[0; 0x150]);
        gameboy.joypad_buttons = 0b1110;
        gameboy.write_u8_addr(0xFF00, 0x30);
        gameboy.mem[INTERRUPT_FLAG_ADDR] = 0;

        gameboy.write_u8_addr(0xFF00, 0x10);

        assert_eq!(gameboy.mem[INTERRUPT_FLAG_ADDR] & 0x10, 0x10);
    }
}
