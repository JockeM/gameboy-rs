use gameboy_rs::registers::{Flag, Register16, Register8};
use gameboy_rs::Gameboy;

#[test]
fn reads_and_writes_memory_words_little_endian_with_wrapping() {
    let mut gameboy = Gameboy::load(&[]);

    gameboy.write_u16_addr(0xC000, 0x1234);
    assert_eq!(gameboy.read_u8_addr(0xC000), 0x34);
    assert_eq!(gameboy.read_u8_addr(0xC001), 0x12);
    assert_eq!(gameboy.read_u16_addr(0xC000), 0x1234);

    gameboy.write_u16_addr(0xFFFF, 0xABCD);
    assert_eq!(gameboy.read_u8_addr(0xFFFF), 0xCD);
    assert_eq!(gameboy.read_u8_addr(0x0000), 0xAB);
    assert_eq!(gameboy.read_u16_addr(0xFFFF), 0xABCD);
}

#[test]
fn pushes_and_pops_stack_words() {
    let mut gameboy = Gameboy::load(&[]);
    gameboy.write_u16(Register16::SP, 0xD000);

    gameboy.push_u16(0xBEEF);

    assert_eq!(gameboy.read_u16(Register16::SP), 0xCFFE);
    assert_eq!(gameboy.read_u8_addr(0xCFFE), 0xEF);
    assert_eq!(gameboy.read_u8_addr(0xCFFF), 0xBE);
    assert_eq!(gameboy.pop_u16(), 0xBEEF);
    assert_eq!(gameboy.read_u16(Register16::SP), 0xD000);
}

#[test]
fn exposes_sp_pc_and_signed_offsets() {
    let mut gameboy = Gameboy::load(&[]);

    assert!(!gameboy.halted);
    assert!(!gameboy.stopped);
    assert!(!gameboy.interrupts_enabled);

    gameboy.write_u16(Register16::SP, 0xFFFE);
    gameboy.write_u16(Register16::PC, 0x1234);

    assert_eq!(gameboy.read_u16(Register16::SP), 0xFFFE);
    assert_eq!(gameboy.read_u16(Register16::PC), 0x1234);
    assert_eq!(Gameboy::signed_e8(0x7F), 127);
    assert_eq!(Gameboy::signed_e8(0x80), -128);
    assert_eq!(Gameboy::add_signed_e8(0x1000, 0x02), 0x1002);
    assert_eq!(Gameboy::add_signed_e8(0x1000, 0xFE), 0x0FFE);
}

#[test]
fn masks_flag_register_low_nibble() {
    let mut gameboy = Gameboy::load(&[]);

    gameboy.write_u16(Register16::AF, 0x12FF);
    assert_eq!(gameboy.read_u16(Register16::AF), 0x12F0);

    gameboy.write_u8(Register8::F, 0xFF);
    assert_eq!(gameboy.read_u8(Register8::F), 0xF0);
}

#[test]
fn alu_helpers_update_accumulator_and_flags() {
    let mut gameboy = Gameboy::load(&[]);

    gameboy.write_u8(Register8::A, 0x0F);
    gameboy.alu_add_a(0x01);
    assert_eq!(gameboy.read_u8(Register8::A), 0x10);
    assert!(!gameboy.read_flag(Flag::Zero));
    assert!(!gameboy.read_flag(Flag::Subtraction));
    assert!(gameboy.read_flag(Flag::HalfCarry));
    assert!(!gameboy.read_flag(Flag::Carry));

    gameboy.write_u8(Register8::A, 0xFF);
    gameboy.write_flag(Flag::Carry, true);
    gameboy.alu_adc_a(0x00);
    assert_eq!(gameboy.read_u8(Register8::A), 0x00);
    assert!(gameboy.read_flag(Flag::Zero));
    assert!(gameboy.read_flag(Flag::HalfCarry));
    assert!(gameboy.read_flag(Flag::Carry));

    gameboy.write_u8(Register8::A, 0x10);
    gameboy.alu_sub_a(0x01);
    assert_eq!(gameboy.read_u8(Register8::A), 0x0F);
    assert!(gameboy.read_flag(Flag::Subtraction));
    assert!(gameboy.read_flag(Flag::HalfCarry));
    assert!(!gameboy.read_flag(Flag::Carry));

    gameboy.write_u8(Register8::A, 0x00);
    gameboy.write_flag(Flag::Carry, true);
    gameboy.alu_sbc_a(0x00);
    assert_eq!(gameboy.read_u8(Register8::A), 0xFF);
    assert!(gameboy.read_flag(Flag::Subtraction));
    assert!(gameboy.read_flag(Flag::HalfCarry));
    assert!(gameboy.read_flag(Flag::Carry));
}

#[test]
fn logic_helpers_update_accumulator_and_flags() {
    let mut gameboy = Gameboy::load(&[]);

    gameboy.write_u8(Register8::A, 0b1010_0000);
    gameboy.alu_and_a(0b0010_0000);
    assert_eq!(gameboy.read_u8(Register8::A), 0b0010_0000);
    assert!(!gameboy.read_flag(Flag::Zero));
    assert!(gameboy.read_flag(Flag::HalfCarry));
    assert!(!gameboy.read_flag(Flag::Carry));

    gameboy.alu_xor_a(0b0010_0000);
    assert_eq!(gameboy.read_u8(Register8::A), 0);
    assert!(gameboy.read_flag(Flag::Zero));
    assert!(!gameboy.read_flag(Flag::HalfCarry));

    gameboy.alu_or_a(0x80);
    assert_eq!(gameboy.read_u8(Register8::A), 0x80);
    assert!(!gameboy.read_flag(Flag::Zero));

    gameboy.alu_cp_a(0x90);
    assert_eq!(gameboy.read_u8(Register8::A), 0x80);
    assert!(gameboy.read_flag(Flag::Subtraction));
    assert!(gameboy.read_flag(Flag::Carry));
}

#[test]
fn cb_prefix_dispatches_prefixed_opcode() {
    let mut rom = vec![0; 0x100];
    rom.extend_from_slice(&[0xCB, 0x11]);
    let mut gameboy = Gameboy::load(&rom);
    gameboy.write_u8(Register8::C, 0x80);

    gameboy.execute();

    assert_eq!(gameboy.pc, 0x102);
    assert_eq!(gameboy.read_u8(Register8::C), 0x00);
    assert!(gameboy.read_flag(Flag::Zero));
    assert!(gameboy.read_flag(Flag::Carry));
}
