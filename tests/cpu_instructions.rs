use gameboy_rs::registers::{Flag, Register16, Register8};
use gameboy_rs::Gameboy;

#[derive(Default)]
struct Expected {
    pc: Option<u16>,
    bc: Option<u16>,
    de: Option<u16>,
    hl: Option<u16>,
    sp: Option<u16>,
    a: Option<u8>,
    b: Option<u8>,
    c: Option<u8>,
    d: Option<u8>,
    e: Option<u8>,
    h: Option<u8>,
    l: Option<u8>,
    mem: &'static [(u16, u8)],
    zero: Option<bool>,
    subtraction: Option<bool>,
    half_carry: Option<bool>,
    carry: Option<bool>,
}

struct Case {
    name: &'static str,
    program: &'static [u8],
    setup: fn(&mut Gameboy),
    expected: Expected,
}

fn no_setup(_: &mut Gameboy) {}

fn run_case(case: &Case) {
    let mut rom = vec![0; 0x100];
    rom.extend_from_slice(case.program);
    let mut gameboy = Gameboy::load(&rom);

    (case.setup)(&mut gameboy);
    gameboy.execute();

    if let Some(pc) = case.expected.pc {
        assert_eq!(gameboy.pc, pc, "{} pc", case.name);
    }
    if let Some(bc) = case.expected.bc {
        assert_eq!(gameboy.read_u16(Register16::BC), bc, "{} bc", case.name);
    }
    if let Some(de) = case.expected.de {
        assert_eq!(gameboy.read_u16(Register16::DE), de, "{} de", case.name);
    }
    if let Some(hl) = case.expected.hl {
        assert_eq!(gameboy.read_u16(Register16::HL), hl, "{} hl", case.name);
    }
    if let Some(sp) = case.expected.sp {
        assert_eq!(gameboy.sp, sp, "{} sp", case.name);
    }
    if let Some(a) = case.expected.a {
        assert_eq!(gameboy.read_u8(Register8::A), a, "{} a", case.name);
    }
    if let Some(b) = case.expected.b {
        assert_eq!(gameboy.read_u8(Register8::B), b, "{} b", case.name);
    }
    if let Some(c) = case.expected.c {
        assert_eq!(gameboy.read_u8(Register8::C), c, "{} c", case.name);
    }
    if let Some(d) = case.expected.d {
        assert_eq!(gameboy.read_u8(Register8::D), d, "{} d", case.name);
    }
    if let Some(e) = case.expected.e {
        assert_eq!(gameboy.read_u8(Register8::E), e, "{} e", case.name);
    }
    if let Some(h) = case.expected.h {
        assert_eq!(gameboy.read_u8(Register8::H), h, "{} h", case.name);
    }
    if let Some(l) = case.expected.l {
        assert_eq!(gameboy.read_u8(Register8::L), l, "{} l", case.name);
    }
    for &(address, value) in case.expected.mem {
        assert_eq!(gameboy.mem[address as usize], value, "{} mem", case.name);
    }
    if let Some(zero) = case.expected.zero {
        assert_eq!(gameboy.read_flag(Flag::Zero), zero, "{} zero", case.name);
    }
    if let Some(subtraction) = case.expected.subtraction {
        assert_eq!(
            gameboy.read_flag(Flag::Subtraction),
            subtraction,
            "{} subtraction",
            case.name
        );
    }
    if let Some(half_carry) = case.expected.half_carry {
        assert_eq!(
            gameboy.read_flag(Flag::HalfCarry),
            half_carry,
            "{} half carry",
            case.name
        );
    }
    if let Some(carry) = case.expected.carry {
        assert_eq!(gameboy.read_flag(Flag::Carry), carry, "{} carry", case.name);
    }
}

#[test]
fn executes_implemented_unprefixed_instructions() {
    let cases = [
        Case {
            name: "00 NOP",
            program: &[0x00],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x101),
                ..Expected::default()
            },
        },
        Case {
            name: "01 LD BC,d16",
            program: &[0x01, 0x34, 0x12],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x103),
                bc: Some(0x1234),
                ..Expected::default()
            },
        },
        Case {
            name: "02 LD (BC),A",
            program: &[0x02],
            setup: |gameboy| {
                gameboy.write_u16(Register16::BC, 0xC000);
                gameboy.write_u8(Register8::A, 0x42);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xC000, 0x42)],
                ..Expected::default()
            },
        },
        Case {
            name: "03 INC BC",
            program: &[0x03],
            setup: |gameboy| gameboy.write_u16(Register16::BC, 0x00FF),
            expected: Expected {
                pc: Some(0x101),
                bc: Some(0x0100),
                ..Expected::default()
            },
        },
        Case {
            name: "04 INC B",
            program: &[0x04],
            setup: |gameboy| gameboy.write_u8(Register8::B, 0x0F),
            expected: Expected {
                pc: Some(0x101),
                b: Some(0x10),
                ..Expected::default()
            },
        },
        Case {
            name: "05 DEC B",
            program: &[0x05],
            setup: |gameboy| gameboy.write_u8(Register8::B, 0x10),
            expected: Expected {
                pc: Some(0x101),
                b: Some(0x0F),
                ..Expected::default()
            },
        },
        Case {
            name: "06 LD B,d8",
            program: &[0x06, 0x7B],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x102),
                b: Some(0x7B),
                ..Expected::default()
            },
        },
        Case {
            name: "07 RLCA",
            program: &[0x07],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x80);
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x01),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "C3 JP a16",
            program: &[0xC3, 0x00, 0x20],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x2000),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_2_load_instructions() {
    let cases = [
        Case {
            name: "08 LD (a16),SP",
            program: &[0x08, 0x00, 0xC0],
            setup: |gameboy| gameboy.sp = 0xBEEF,
            expected: Expected {
                pc: Some(0x103),
                mem: &[(0xC000, 0xEF), (0xC001, 0xBE)],
                ..Expected::default()
            },
        },
        Case {
            name: "0A LD A,(BC)",
            program: &[0x0A],
            setup: |gameboy| {
                gameboy.write_u16(Register16::BC, 0xC123);
                gameboy.write_u8_addr(0xC123, 0x42);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x42),
                ..Expected::default()
            },
        },
        Case {
            name: "0E LD C,n8",
            program: &[0x0E, 0x91],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x102),
                c: Some(0x91),
                ..Expected::default()
            },
        },
        Case {
            name: "11 LD DE,n16",
            program: &[0x11, 0x34, 0x12],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x103),
                de: Some(0x1234),
                ..Expected::default()
            },
        },
        Case {
            name: "12 LD (DE),A",
            program: &[0x12],
            setup: |gameboy| {
                gameboy.write_u16(Register16::DE, 0xC200);
                gameboy.write_u8(Register8::A, 0x55);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xC200, 0x55)],
                ..Expected::default()
            },
        },
        Case {
            name: "16 LD D,n8",
            program: &[0x16, 0xD0],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x102),
                d: Some(0xD0),
                ..Expected::default()
            },
        },
        Case {
            name: "1A LD A,(DE)",
            program: &[0x1A],
            setup: |gameboy| {
                gameboy.write_u16(Register16::DE, 0xC234);
                gameboy.write_u8_addr(0xC234, 0x66);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x66),
                ..Expected::default()
            },
        },
        Case {
            name: "1E LD E,n8",
            program: &[0x1E, 0xE1],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x102),
                e: Some(0xE1),
                ..Expected::default()
            },
        },
        Case {
            name: "21 LD HL,n16",
            program: &[0x21, 0x78, 0x56],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x103),
                hl: Some(0x5678),
                ..Expected::default()
            },
        },
        Case {
            name: "22 LD (HL+),A",
            program: &[0x22],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC300);
                gameboy.write_u8(Register8::A, 0x77);
            },
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0xC301),
                mem: &[(0xC300, 0x77)],
                ..Expected::default()
            },
        },
        Case {
            name: "26 LD H,n8",
            program: &[0x26, 0xAB],
            setup: |gameboy| gameboy.write_u8(Register8::L, 0xCD),
            expected: Expected {
                pc: Some(0x102),
                hl: Some(0xABCD),
                h: Some(0xAB),
                ..Expected::default()
            },
        },
        Case {
            name: "2A LD A,(HL+)",
            program: &[0x2A],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC301);
                gameboy.write_u8_addr(0xC301, 0x88);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x88),
                hl: Some(0xC302),
                ..Expected::default()
            },
        },
        Case {
            name: "2E LD L,n8",
            program: &[0x2E, 0xCD],
            setup: |gameboy| gameboy.write_u8(Register8::H, 0xAB),
            expected: Expected {
                pc: Some(0x102),
                hl: Some(0xABCD),
                l: Some(0xCD),
                ..Expected::default()
            },
        },
        Case {
            name: "31 LD SP,n16",
            program: &[0x31, 0xFE, 0xFF],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x103),
                sp: Some(0xFFFE),
                ..Expected::default()
            },
        },
        Case {
            name: "32 LD (HL-),A",
            program: &[0x32],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC400);
                gameboy.write_u8(Register8::A, 0x99);
            },
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0xC3FF),
                mem: &[(0xC400, 0x99)],
                ..Expected::default()
            },
        },
        Case {
            name: "36 LD (HL),n8",
            program: &[0x36, 0xA5],
            setup: |gameboy| gameboy.write_u16(Register16::HL, 0xC500),
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC500, 0xA5)],
                ..Expected::default()
            },
        },
        Case {
            name: "3A LD A,(HL-)",
            program: &[0x3A],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC501);
                gameboy.write_u8_addr(0xC501, 0xB6);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0xB6),
                hl: Some(0xC500),
                ..Expected::default()
            },
        },
        Case {
            name: "3E LD A,n8",
            program: &[0x3E, 0xC7],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x102),
                a: Some(0xC7),
                ..Expected::default()
            },
        },
        Case {
            name: "40 LD B,B",
            program: &[0x40],
            setup: |gameboy| gameboy.write_u8(Register8::B, 0x12),
            expected: Expected {
                pc: Some(0x101),
                b: Some(0x12),
                ..Expected::default()
            },
        },
        Case {
            name: "4F LD C,A",
            program: &[0x4F],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x34),
            expected: Expected {
                pc: Some(0x101),
                c: Some(0x34),
                ..Expected::default()
            },
        },
        Case {
            name: "56 LD D,(HL)",
            program: &[0x56],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC600);
                gameboy.write_u8_addr(0xC600, 0x45);
            },
            expected: Expected {
                pc: Some(0x101),
                d: Some(0x45),
                ..Expected::default()
            },
        },
        Case {
            name: "71 LD (HL),C",
            program: &[0x71],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC601);
                gameboy.write_u8(Register8::C, 0x56);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xC601, 0x56)],
                ..Expected::default()
            },
        },
        Case {
            name: "7E LD A,(HL)",
            program: &[0x7E],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC602);
                gameboy.write_u8_addr(0xC602, 0x67);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x67),
                ..Expected::default()
            },
        },
        Case {
            name: "77 LD (HL),A",
            program: &[0x77],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC603);
                gameboy.write_u8(Register8::A, 0x68);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xC603, 0x68)],
                ..Expected::default()
            },
        },
        Case {
            name: "7F LD A,A",
            program: &[0x7F],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x69),
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x69),
                ..Expected::default()
            },
        },
        Case {
            name: "E0 LDH (a8),A",
            program: &[0xE0, 0x80],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x78),
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xFF80, 0x78)],
                ..Expected::default()
            },
        },
        Case {
            name: "E2 LDH (C),A",
            program: &[0xE2],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x89);
                gameboy.write_u8(Register8::C, 0x81);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xFF81, 0x89)],
                ..Expected::default()
            },
        },
        Case {
            name: "EA LD (a16),A",
            program: &[0xEA, 0x03, 0xC6],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x9A),
            expected: Expected {
                pc: Some(0x103),
                mem: &[(0xC603, 0x9A)],
                ..Expected::default()
            },
        },
        Case {
            name: "F0 LDH A,(a8)",
            program: &[0xF0, 0x82],
            setup: |gameboy| gameboy.write_u8_addr(0xFF82, 0xAB),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0xAB),
                ..Expected::default()
            },
        },
        Case {
            name: "F2 LDH A,(C)",
            program: &[0xF2],
            setup: |gameboy| {
                gameboy.write_u8(Register8::C, 0x83);
                gameboy.write_u8_addr(0xFF83, 0xBC);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0xBC),
                ..Expected::default()
            },
        },
        Case {
            name: "FA LD A,(a16)",
            program: &[0xFA, 0x04, 0xC6],
            setup: |gameboy| gameboy.write_u8_addr(0xC604, 0xCD),
            expected: Expected {
                pc: Some(0x103),
                a: Some(0xCD),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_3_16_bit_arithmetic_and_register_movement() {
    let cases = [
        Case {
            name: "09 ADD HL,BC",
            program: &[0x09],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0x0FFF);
                gameboy.write_u16(Register16::BC, 0x0001);
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
            },
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0x1000),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "0B DEC BC",
            program: &[0x0B],
            setup: |gameboy| gameboy.write_u16(Register16::BC, 0x0000),
            expected: Expected {
                pc: Some(0x101),
                bc: Some(0xFFFF),
                ..Expected::default()
            },
        },
        Case {
            name: "13 INC DE",
            program: &[0x13],
            setup: |gameboy| gameboy.write_u16(Register16::DE, 0x00FF),
            expected: Expected {
                pc: Some(0x101),
                de: Some(0x0100),
                ..Expected::default()
            },
        },
        Case {
            name: "19 ADD HL,DE",
            program: &[0x19],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xFFFF);
                gameboy.write_u16(Register16::DE, 0x0001);
            },
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0x0000),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "1B DEC DE",
            program: &[0x1B],
            setup: |gameboy| gameboy.write_u16(Register16::DE, 0x0100),
            expected: Expected {
                pc: Some(0x101),
                de: Some(0x00FF),
                ..Expected::default()
            },
        },
        Case {
            name: "23 INC HL",
            program: &[0x23],
            setup: |gameboy| gameboy.write_u16(Register16::HL, 0xFFFF),
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0x0000),
                ..Expected::default()
            },
        },
        Case {
            name: "29 ADD HL,HL",
            program: &[0x29],
            setup: |gameboy| gameboy.write_u16(Register16::HL, 0x8000),
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0x0000),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "2B DEC HL",
            program: &[0x2B],
            setup: |gameboy| gameboy.write_u16(Register16::HL, 0x0000),
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0xFFFF),
                ..Expected::default()
            },
        },
        Case {
            name: "33 INC SP",
            program: &[0x33],
            setup: |gameboy| gameboy.sp = 0xFFFF,
            expected: Expected {
                pc: Some(0x101),
                sp: Some(0x0000),
                ..Expected::default()
            },
        },
        Case {
            name: "39 ADD HL,SP",
            program: &[0x39],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0x8FFF);
                gameboy.sp = 0x8001;
            },
            expected: Expected {
                pc: Some(0x101),
                hl: Some(0x1000),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "3B DEC SP",
            program: &[0x3B],
            setup: |gameboy| gameboy.sp = 0x0000,
            expected: Expected {
                pc: Some(0x101),
                sp: Some(0xFFFF),
                ..Expected::default()
            },
        },
        Case {
            name: "E8 ADD SP,e8 positive",
            program: &[0xE8, 0x01],
            setup: |gameboy| gameboy.sp = 0x00FF,
            expected: Expected {
                pc: Some(0x102),
                sp: Some(0x0100),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "E8 ADD SP,e8 negative",
            program: &[0xE8, 0xFE],
            setup: |gameboy| gameboy.sp = 0x0001,
            expected: Expected {
                pc: Some(0x102),
                sp: Some(0xFFFF),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "F8 LD HL,SP+e8",
            program: &[0xF8, 0x02],
            setup: |gameboy| gameboy.sp = 0x00FE,
            expected: Expected {
                pc: Some(0x102),
                hl: Some(0x0100),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "F9 LD SP,HL",
            program: &[0xF9],
            setup: |gameboy| gameboy.write_u16(Register16::HL, 0xC123),
            expected: Expected {
                pc: Some(0x101),
                sp: Some(0xC123),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_4_8_bit_inc_dec_and_accumulator_control() {
    let cases = [
        Case {
            name: "0C INC C sets half-carry and preserves carry",
            program: &[0x0C],
            setup: |gameboy| {
                gameboy.write_u8(Register8::C, 0x0F);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                c: Some(0x10),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "0D DEC C sets borrow from bit 4 and preserves carry",
            program: &[0x0D],
            setup: |gameboy| {
                gameboy.write_u8(Register8::C, 0x10);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                c: Some(0x0F),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "0F RRCA rotates bit 0 into bit 7 and carry",
            program: &[0x0F],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x01);
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x80),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "14 INC D wraps to zero",
            program: &[0x14],
            setup: |gameboy| gameboy.write_u8(Register8::D, 0xFF),
            expected: Expected {
                pc: Some(0x101),
                d: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "15 DEC D sets zero",
            program: &[0x15],
            setup: |gameboy| gameboy.write_u8(Register8::D, 0x01),
            expected: Expected {
                pc: Some(0x101),
                d: Some(0x00),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "17 RLA rotates through carry",
            program: &[0x17],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x80);
                gameboy.write_flag(Flag::Carry, true);
                gameboy.write_flag(Flag::Zero, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x01),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "1C INC E",
            program: &[0x1C],
            setup: |gameboy| {
                gameboy.write_u8(Register8::E, 0x01);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                e: Some(0x02),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "1D DEC E wraps and borrows from bit 4",
            program: &[0x1D],
            setup: |gameboy| gameboy.write_u8(Register8::E, 0x00),
            expected: Expected {
                pc: Some(0x101),
                e: Some(0xFF),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "1F RRA rotates through carry",
            program: &[0x1F],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x01);
                gameboy.write_flag(Flag::Carry, true);
                gameboy.write_flag(Flag::Zero, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x80),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "24 INC H",
            program: &[0x24],
            setup: |gameboy| gameboy.write_u8(Register8::H, 0x2F),
            expected: Expected {
                pc: Some(0x101),
                h: Some(0x30),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "25 DEC H",
            program: &[0x25],
            setup: |gameboy| gameboy.write_u8(Register8::H, 0x20),
            expected: Expected {
                pc: Some(0x101),
                h: Some(0x1F),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "27 DAA after addition",
            program: &[0x27],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x3C);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x42),
                zero: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "27 DAA after addition sets carry and zero",
            program: &[0x27],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x9A),
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "27 DAA after subtraction",
            program: &[0x27],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x0F);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x09),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "27 DAA after subtraction preserves carry",
            program: &[0x27],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x9A);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, false);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x3A),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "2C INC L",
            program: &[0x2C],
            setup: |gameboy| gameboy.write_u8(Register8::L, 0xFF),
            expected: Expected {
                pc: Some(0x101),
                l: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "2D DEC L",
            program: &[0x2D],
            setup: |gameboy| gameboy.write_u8(Register8::L, 0x01),
            expected: Expected {
                pc: Some(0x101),
                l: Some(0x00),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "2F CPL complements A and preserves zero/carry",
            program: &[0x2F],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x35);
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0xCA),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "34 INC (HL)",
            program: &[0x34],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC700);
                gameboy.write_u8_addr(0xC700, 0x0F);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xC700, 0x10)],
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "35 DEC (HL)",
            program: &[0x35],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC701);
                gameboy.write_u8_addr(0xC701, 0x10);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                mem: &[(0xC701, 0x0F)],
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "37 SCF sets carry and preserves zero",
            program: &[0x37],
            setup: |gameboy| {
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "3C INC A",
            program: &[0x3C],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x0F),
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x10),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "3D DEC A",
            program: &[0x3D],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x10),
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x0F),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "3F CCF complements carry and preserves zero",
            program: &[0x3F],
            setup: |gameboy| {
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_5_alu_register_memory_and_immediate_groups() {
    let cases = [
        Case {
            name: "80 ADD A,B sets half-carry",
            program: &[0x80],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x0F);
                gameboy.write_u8(Register8::B, 0x01);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x10),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "86 ADD A,(HL) wraps with carry",
            program: &[0x86],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0xF0);
                gameboy.write_u16(Register16::HL, 0xC800);
                gameboy.write_u8_addr(0xC800, 0x10);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "88 ADC A,B includes carry",
            program: &[0x88],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x0F);
                gameboy.write_u8(Register8::B, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x10),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "8E ADC A,(HL) wraps with carry",
            program: &[0x8E],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0xFF);
                gameboy.write_u16(Register16::HL, 0xC801);
                gameboy.write_u8_addr(0xC801, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "90 SUB A,B sets subtraction and half-borrow",
            program: &[0x90],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x10);
                gameboy.write_u8(Register8::B, 0x01);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x0F),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "96 SUB A,(HL) borrows to zero",
            program: &[0x96],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x00);
                gameboy.write_u16(Register16::HL, 0xC802);
                gameboy.write_u8_addr(0xC802, 0x01);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0xFF),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "98 SBC A,B includes carry",
            program: &[0x98],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x10);
                gameboy.write_u8(Register8::B, 0x0F);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "9E SBC A,(HL) borrows with carry",
            program: &[0x9E],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x00);
                gameboy.write_u16(Register16::HL, 0xC803);
                gameboy.write_u8_addr(0xC803, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0xFF),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "A0 AND A,B sets half-carry",
            program: &[0xA0],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0xF0);
                gameboy.write_u8(Register8::B, 0x0F);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "A6 AND A,(HL)",
            program: &[0xA6],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0xF3);
                gameboy.write_u16(Register16::HL, 0xC804);
                gameboy.write_u8_addr(0xC804, 0x3C);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x30),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "A8 XOR A,B clears flags",
            program: &[0xA8],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x55);
                gameboy.write_u8(Register8::B, 0xFF);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0xAA),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "AE XOR A,(HL) sets zero",
            program: &[0xAE],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x5A);
                gameboy.write_u16(Register16::HL, 0xC805);
                gameboy.write_u8_addr(0xC805, 0x5A);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "B0 OR A,B clears flags",
            program: &[0xB0],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x50);
                gameboy.write_u8(Register8::B, 0x0A);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x5A),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "B6 OR A,(HL) sets zero",
            program: &[0xB6],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x00);
                gameboy.write_u16(Register16::HL, 0xC806);
                gameboy.write_u8_addr(0xC806, 0x00);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "B8 CP A,B sets compare flags without changing A",
            program: &[0xB8],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x10);
                gameboy.write_u8(Register8::B, 0x01);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x10),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "BE CP A,(HL) sets zero without changing A",
            program: &[0xBE],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x42);
                gameboy.write_u16(Register16::HL, 0xC807);
                gameboy.write_u8_addr(0xC807, 0x42);
            },
            expected: Expected {
                pc: Some(0x101),
                a: Some(0x42),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "C6 ADD A,n8",
            program: &[0xC6, 0x01],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0xFF),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CE ADC A,n8",
            program: &[0xCE, 0x10],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x0F);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x20),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "D6 SUB A,n8",
            program: &[0xD6, 0x01],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x00),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0xFF),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "DE SBC A,n8",
            program: &[0xDE, 0x0F],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x10);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "E6 AND A,n8",
            program: &[0xE6, 0x0F],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0xF0),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "EE XOR A,n8",
            program: &[0xEE, 0xFF],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x55),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0xAA),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "F6 OR A,n8",
            program: &[0xF6, 0x0A],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x50),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x5A),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "FE CP A,n8 does not change A",
            program: &[0xFE, 0x43],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x42),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x42),
                zero: Some(false),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_6_relative_and_absolute_jumps() {
    let cases = [
        Case {
            name: "18 JR e8 positive offset",
            program: &[0x18, 0x05],
            setup: |gameboy| {
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x107),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "18 JR e8 negative offset",
            program: &[0x18, 0xFC],
            setup: no_setup,
            expected: Expected {
                pc: Some(0x0FE),
                ..Expected::default()
            },
        },
        Case {
            name: "20 JR NZ,e8 taken",
            program: &[0x20, 0x02],
            setup: |gameboy| gameboy.write_flag(Flag::Zero, false),
            expected: Expected {
                pc: Some(0x104),
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "20 JR NZ,e8 not taken",
            program: &[0x20, 0x02],
            setup: |gameboy| gameboy.write_flag(Flag::Zero, true),
            expected: Expected {
                pc: Some(0x102),
                zero: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "28 JR Z,e8 taken",
            program: &[0x28, 0x02],
            setup: |gameboy| gameboy.write_flag(Flag::Zero, true),
            expected: Expected {
                pc: Some(0x104),
                zero: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "30 JR NC,e8 not taken",
            program: &[0x30, 0x02],
            setup: |gameboy| gameboy.write_flag(Flag::Carry, true),
            expected: Expected {
                pc: Some(0x102),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "38 JR C,e8 taken",
            program: &[0x38, 0xFE],
            setup: |gameboy| gameboy.write_flag(Flag::Carry, true),
            expected: Expected {
                pc: Some(0x100),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "C2 JP NZ,a16 taken",
            program: &[0xC2, 0x34, 0x12],
            setup: |gameboy| gameboy.write_flag(Flag::Zero, false),
            expected: Expected {
                pc: Some(0x1234),
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CA JP Z,a16 not taken",
            program: &[0xCA, 0x34, 0x12],
            setup: |gameboy| gameboy.write_flag(Flag::Zero, false),
            expected: Expected {
                pc: Some(0x103),
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "D2 JP NC,a16 taken",
            program: &[0xD2, 0x78, 0x56],
            setup: |gameboy| gameboy.write_flag(Flag::Carry, false),
            expected: Expected {
                pc: Some(0x5678),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "DA JP C,a16 not taken",
            program: &[0xDA, 0x78, 0x56],
            setup: |gameboy| gameboy.write_flag(Flag::Carry, false),
            expected: Expected {
                pc: Some(0x103),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "E9 JP HL",
            program: &[0xE9],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC123);
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, false);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0xC123),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_7_push_and_pop_register_pairs() {
    let push_cases = [
        (0xC5, Register16::BC, 0x1234, 0x34, 0x12),
        (0xD5, Register16::DE, 0x5678, 0x78, 0x56),
        (0xE5, Register16::HL, 0x9ABC, 0xBC, 0x9A),
        (0xF5, Register16::AF, 0xDEF0, 0xF0, 0xDE),
    ];

    for (opcode, register, value, lo, hi) in push_cases {
        let mut rom = vec![0; 0x100];
        rom.push(opcode);
        let mut gameboy = Gameboy::load(&rom);
        gameboy.sp = 0xD000;
        gameboy.write_u16(register, value);

        gameboy.execute();

        assert_eq!(gameboy.sp, 0xCFFE, "{opcode:#04X} sp");
        assert_eq!(gameboy.pc, 0x101, "{opcode:#04X} pc");
        assert_eq!(gameboy.mem[0xCFFE], lo, "{opcode:#04X} low byte");
        assert_eq!(gameboy.mem[0xCFFF], hi, "{opcode:#04X} high byte");
    }

    let pop_cases = [
        (0xC1, Register16::BC, 0x1234, 0x34, 0x12),
        (0xD1, Register16::DE, 0x5678, 0x78, 0x56),
        (0xE1, Register16::HL, 0x9ABC, 0xBC, 0x9A),
    ];

    for (opcode, register, value, lo, hi) in pop_cases {
        let mut rom = vec![0; 0x100];
        rom.push(opcode);
        let mut gameboy = Gameboy::load(&rom);
        gameboy.sp = 0xCFFE;
        gameboy.write_u8_addr(0xCFFE, lo);
        gameboy.write_u8_addr(0xCFFF, hi);

        gameboy.execute();

        assert_eq!(gameboy.sp, 0xD000, "{opcode:#04X} sp");
        assert_eq!(gameboy.pc, 0x101, "{opcode:#04X} pc");
        assert_eq!(gameboy.read_u16(register), value, "{opcode:#04X} register");
    }
}

#[test]
fn pop_af_masks_low_flag_nibble() {
    let mut rom = vec![0; 0x100];
    rom.push(0xF1);
    let mut gameboy = Gameboy::load(&rom);
    gameboy.sp = 0xCFFE;
    gameboy.write_u8_addr(0xCFFE, 0xFF);
    gameboy.write_u8_addr(0xCFFF, 0x12);

    gameboy.execute();

    assert_eq!(gameboy.sp, 0xD000);
    assert_eq!(gameboy.read_u16(Register16::AF), 0x12F0);
}

#[test]
fn executes_phase_7_call_ret_and_reti() {
    let cases = [
        Case {
            name: "CD CALL a16",
            program: &[0xCD, 0x34, 0x12],
            setup: |gameboy| gameboy.sp = 0xD000,
            expected: Expected {
                pc: Some(0x1234),
                sp: Some(0xCFFE),
                mem: &[(0xCFFE, 0x03), (0xCFFF, 0x01)],
                ..Expected::default()
            },
        },
        Case {
            name: "C9 RET",
            program: &[0xC9],
            setup: |gameboy| {
                gameboy.sp = 0xCFFE;
                gameboy.write_u16_addr(0xCFFE, 0x4567);
            },
            expected: Expected {
                pc: Some(0x4567),
                sp: Some(0xD000),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }

    let mut rom = vec![0; 0x100];
    rom.push(0xD9);
    let mut gameboy = Gameboy::load(&rom);
    gameboy.sp = 0xCFFE;
    gameboy.write_u16_addr(0xCFFE, 0x2345);

    gameboy.execute();

    assert_eq!(gameboy.pc, 0x2345);
    assert_eq!(gameboy.sp, 0xD000);
    assert!(gameboy.interrupts_enabled);
}

#[test]
fn executes_phase_7_conditional_calls_and_returns() {
    let cases = [
        Case {
            name: "C4 CALL NZ,a16 taken",
            program: &[0xC4, 0x00, 0x20],
            setup: |gameboy| {
                gameboy.sp = 0xD000;
                gameboy.write_flag(Flag::Zero, false);
            },
            expected: Expected {
                pc: Some(0x2000),
                sp: Some(0xCFFE),
                mem: &[(0xCFFE, 0x03), (0xCFFF, 0x01)],
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CC CALL Z,a16 not taken",
            program: &[0xCC, 0x00, 0x20],
            setup: |gameboy| {
                gameboy.sp = 0xD000;
                gameboy.write_flag(Flag::Zero, false);
            },
            expected: Expected {
                pc: Some(0x103),
                sp: Some(0xD000),
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "D4 CALL NC,a16 taken",
            program: &[0xD4, 0x00, 0x30],
            setup: |gameboy| {
                gameboy.sp = 0xD000;
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x3000),
                sp: Some(0xCFFE),
                mem: &[(0xCFFE, 0x03), (0xCFFF, 0x01)],
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "DC CALL C,a16 not taken",
            program: &[0xDC, 0x00, 0x30],
            setup: |gameboy| {
                gameboy.sp = 0xD000;
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x103),
                sp: Some(0xD000),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "C0 RET NZ taken",
            program: &[0xC0],
            setup: |gameboy| {
                gameboy.sp = 0xCFFE;
                gameboy.write_u16_addr(0xCFFE, 0x2222);
                gameboy.write_flag(Flag::Zero, false);
            },
            expected: Expected {
                pc: Some(0x2222),
                sp: Some(0xD000),
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "C8 RET Z not taken",
            program: &[0xC8],
            setup: |gameboy| {
                gameboy.sp = 0xCFFE;
                gameboy.write_u16_addr(0xCFFE, 0x2222);
                gameboy.write_flag(Flag::Zero, false);
            },
            expected: Expected {
                pc: Some(0x101),
                sp: Some(0xCFFE),
                zero: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "D0 RET NC taken",
            program: &[0xD0],
            setup: |gameboy| {
                gameboy.sp = 0xCFFE;
                gameboy.write_u16_addr(0xCFFE, 0x3333);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x3333),
                sp: Some(0xD000),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "D8 RET C not taken",
            program: &[0xD8],
            setup: |gameboy| {
                gameboy.sp = 0xCFFE;
                gameboy.write_u16_addr(0xCFFE, 0x3333);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x101),
                sp: Some(0xCFFE),
                carry: Some(false),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_8_control_and_interrupt_instructions() {
    let mut rom = vec![0; 0x100];
    rom.extend_from_slice(&[0x10, 0x42]);
    let mut gameboy = Gameboy::load(&rom);

    gameboy.execute();

    assert!(gameboy.stopped);
    assert_eq!(gameboy.pc, 0x102);

    let mut rom = vec![0; 0x100];
    rom.push(0x76);
    let mut gameboy = Gameboy::load(&rom);

    gameboy.execute();

    assert!(gameboy.halted);
    assert_eq!(gameboy.pc, 0x101);

    let mut rom = vec![0; 0x100];
    rom.push(0xF3);
    let mut gameboy = Gameboy::load(&rom);
    gameboy.interrupts_enabled = true;

    gameboy.execute();

    assert!(!gameboy.interrupts_enabled);
    assert_eq!(gameboy.pc, 0x101);

    let mut rom = vec![0; 0x100];
    rom.push(0xFB);
    let mut gameboy = Gameboy::load(&rom);

    gameboy.execute();

    assert!(gameboy.interrupts_enabled);
    assert_eq!(gameboy.pc, 0x101);
}

#[test]
fn executes_phase_9_cb_rotate_shift_and_swap_instructions() {
    let cases = [
        Case {
            name: "CB 00 RLC B rotates bit 7 into bit 0 and carry",
            program: &[0xCB, 0x00],
            setup: |gameboy| {
                gameboy.write_u8(Register8::B, 0x80);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                b: Some(0x01),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 06 RLC (HL) sets zero",
            program: &[0xCB, 0x06],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC900);
                gameboy.write_u8_addr(0xC900, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC900, 0x00)],
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 09 RRC C rotates bit 0 into bit 7 and carry",
            program: &[0xCB, 0x09],
            setup: |gameboy| gameboy.write_u8(Register8::C, 0x01),
            expected: Expected {
                pc: Some(0x102),
                c: Some(0x80),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 0E RRC (HL) sets zero",
            program: &[0xCB, 0x0E],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC901);
                gameboy.write_u8_addr(0xC901, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC901, 0x00)],
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 12 RL D rotates through carry",
            program: &[0xCB, 0x12],
            setup: |gameboy| {
                gameboy.write_u8(Register8::D, 0x80);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                d: Some(0x01),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 16 RL (HL) sets zero",
            program: &[0xCB, 0x16],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC902);
                gameboy.write_u8_addr(0xC902, 0x00);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC902, 0x00)],
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 1B RR E rotates through carry",
            program: &[0xCB, 0x1B],
            setup: |gameboy| {
                gameboy.write_u8(Register8::E, 0x01);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                e: Some(0x80),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 1E RR (HL) sets zero",
            program: &[0xCB, 0x1E],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC903);
                gameboy.write_u8_addr(0xC903, 0x00);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC903, 0x00)],
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 24 SLA H shifts bit 7 into carry",
            program: &[0xCB, 0x24],
            setup: |gameboy| gameboy.write_u8(Register8::H, 0x80),
            expected: Expected {
                pc: Some(0x102),
                h: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 26 SLA (HL) shifts left",
            program: &[0xCB, 0x26],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC904);
                gameboy.write_u8_addr(0xC904, 0x81);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC904, 0x02)],
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 2D SRA L preserves sign bit",
            program: &[0xCB, 0x2D],
            setup: |gameboy| gameboy.write_u8(Register8::L, 0x81),
            expected: Expected {
                pc: Some(0x102),
                l: Some(0xC0),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 2E SRA (HL) sets zero and carry",
            program: &[0xCB, 0x2E],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC905);
                gameboy.write_u8_addr(0xC905, 0x01);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC905, 0x00)],
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 37 SWAP A swaps nibbles and clears carry",
            program: &[0xCB, 0x37],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0xF0);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x0F),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 36 SWAP (HL) sets zero",
            program: &[0xCB, 0x36],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC906);
                gameboy.write_u8_addr(0xC906, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC906, 0x00)],
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 38 SRL B shifts bit 0 into carry",
            program: &[0xCB, 0x38],
            setup: |gameboy| gameboy.write_u8(Register8::B, 0x01),
            expected: Expected {
                pc: Some(0x102),
                b: Some(0x00),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 3E SRL (HL) clears bit 7",
            program: &[0xCB, 0x3E],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC907);
                gameboy.write_u8_addr(0xC907, 0x80);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC907, 0x40)],
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(false),
                carry: Some(false),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_10_cb_bit_test_instructions() {
    let cases = [
        Case {
            name: "CB 40 BIT 0,B sets zero when bit is clear and preserves carry",
            program: &[0xCB, 0x40],
            setup: |gameboy| {
                gameboy.write_u8(Register8::B, 0xFE);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                b: Some(0xFE),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 4F BIT 1,A clears zero when bit is set and preserves carry clear",
            program: &[0xCB, 0x4F],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x02);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, false);
                gameboy.write_flag(Flag::Carry, false);
            },
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x02),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(false),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 56 BIT 2,(HL) tests memory without modifying it",
            program: &[0xCB, 0x56],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC908);
                gameboy.write_u8_addr(0xC908, 0x04);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC908, 0x04)],
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 5B BIT 3,E sets zero for clear register bit",
            program: &[0xCB, 0x5B],
            setup: |gameboy| gameboy.write_u8(Register8::E, 0xF7),
            expected: Expected {
                pc: Some(0x102),
                e: Some(0xF7),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 64 BIT 4,H clears zero for set register bit",
            program: &[0xCB, 0x64],
            setup: |gameboy| gameboy.write_u8(Register8::H, 0x10),
            expected: Expected {
                pc: Some(0x102),
                h: Some(0x10),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 69 BIT 5,C sets zero for clear register bit",
            program: &[0xCB, 0x69],
            setup: |gameboy| gameboy.write_u8(Register8::C, 0xDF),
            expected: Expected {
                pc: Some(0x102),
                c: Some(0xDF),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 72 BIT 6,D clears zero for set register bit",
            program: &[0xCB, 0x72],
            setup: |gameboy| gameboy.write_u8(Register8::D, 0x40),
            expected: Expected {
                pc: Some(0x102),
                d: Some(0x40),
                zero: Some(false),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 7D BIT 7,L sets zero for clear register bit",
            program: &[0xCB, 0x7D],
            setup: |gameboy| gameboy.write_u8(Register8::L, 0x7F),
            expected: Expected {
                pc: Some(0x102),
                l: Some(0x7F),
                zero: Some(true),
                subtraction: Some(false),
                half_carry: Some(true),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_11_cb_reset_and_set_instructions() {
    let cases = [
        Case {
            name: "CB 80 RES 0,B clears bit and preserves flags",
            program: &[0xCB, 0x80],
            setup: |gameboy| {
                gameboy.write_u8(Register8::B, 0xFF);
                gameboy.write_flag(Flag::Zero, true);
                gameboy.write_flag(Flag::Subtraction, true);
                gameboy.write_flag(Flag::HalfCarry, true);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                b: Some(0xFE),
                zero: Some(true),
                subtraction: Some(true),
                half_carry: Some(true),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB 9E RES 3,(HL) clears memory bit",
            program: &[0xCB, 0x9E],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC909);
                gameboy.write_u8_addr(0xC909, 0xFF);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC909, 0xF7)],
                ..Expected::default()
            },
        },
        Case {
            name: "CB BD RES 7,L clears high bit",
            program: &[0xCB, 0xBD],
            setup: |gameboy| gameboy.write_u8(Register8::L, 0x80),
            expected: Expected {
                pc: Some(0x102),
                l: Some(0x00),
                ..Expected::default()
            },
        },
        Case {
            name: "CB C7 SET 0,A sets low bit and preserves flags",
            program: &[0xCB, 0xC7],
            setup: |gameboy| {
                gameboy.write_u8(Register8::A, 0x00);
                gameboy.write_flag(Flag::Carry, true);
            },
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x01),
                carry: Some(true),
                ..Expected::default()
            },
        },
        Case {
            name: "CB E6 SET 4,(HL) sets memory bit",
            program: &[0xCB, 0xE6],
            setup: |gameboy| {
                gameboy.write_u16(Register16::HL, 0xC90A);
                gameboy.write_u8_addr(0xC90A, 0x00);
            },
            expected: Expected {
                pc: Some(0x102),
                mem: &[(0xC90A, 0x10)],
                ..Expected::default()
            },
        },
        Case {
            name: "CB FF SET 7,A sets high bit",
            program: &[0xCB, 0xFF],
            setup: |gameboy| gameboy.write_u8(Register8::A, 0x01),
            expected: Expected {
                pc: Some(0x102),
                a: Some(0x81),
                ..Expected::default()
            },
        },
    ];

    for case in cases {
        run_case(&case);
    }
}

#[test]
fn executes_phase_7_rst_vectors() {
    let cases = [
        (0xC7, 0x0000),
        (0xCF, 0x0008),
        (0xD7, 0x0010),
        (0xDF, 0x0018),
        (0xE7, 0x0020),
        (0xEF, 0x0028),
        (0xF7, 0x0030),
        (0xFF, 0x0038),
    ];

    for (opcode, vector) in cases {
        let mut rom = vec![0; 0x100];
        rom.push(opcode);
        let mut gameboy = Gameboy::load(&rom);
        gameboy.sp = 0xD000;

        gameboy.execute();

        assert_eq!(gameboy.pc, vector, "{opcode:#04X} pc");
        assert_eq!(gameboy.sp, 0xCFFE, "{opcode:#04X} sp");
        assert_eq!(gameboy.mem[0xCFFE], 0x01, "{opcode:#04X} low byte");
        assert_eq!(gameboy.mem[0xCFFF], 0x01, "{opcode:#04X} high byte");
    }
}
