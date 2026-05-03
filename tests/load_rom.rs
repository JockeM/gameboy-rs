use std::fs;

use gameboy_rs::Gameboy;

#[test]
fn loads_simple_gb_file_into_memory() {
    let rom_path =
        std::env::temp_dir().join(format!("gameboy-rs-simple-{}.gb", std::process::id()));
    let mut rom = vec![0; 0x101];
    rom[0x100] = 0x00;
    rom.extend_from_slice(&[0; 0x150 - 0x101]);
    rom[0x134..0x13a].copy_from_slice(b"SIMPLE");

    fs::write(&rom_path, &rom).expect("write simple ROM");

    let gameboy = Gameboy::load_file(&rom_path).expect("load simple ROM");

    fs::remove_file(&rom_path).expect("remove simple ROM");

    assert_eq!(gameboy.pc, 0x100);
    assert_eq!(gameboy.sp, 0xFFFE);
    assert_eq!(gameboy.mem[0x0000], 0x00);
    assert_eq!(gameboy.mem[0x0100], 0x00);
    assert_eq!(&gameboy.mem[0x0134..0x013a], b"SIMPLE");
}

#[test]
fn runs_a_frame_and_renders_rom_written_background_tiles() {
    let mut rom = vec![0; 0x100];
    rom.extend_from_slice(&[
        0x21, 0x00, 0x80, // LD HL,$8000
        0x3E, 0xFF, // LD A,$FF
        0x22, // LD (HL+),A ; tile 0 row 0 low bits
        0x3E, 0x00, // LD A,$00
        0x22, // LD (HL+),A ; tile 0 row 0 high bits
        0x21, 0x00, 0x98, // LD HL,$9800
        0x36, 0x00, // LD (HL),$00
        0x76, // HALT
    ]);
    let mut gameboy = Gameboy::load(&rom);
    gameboy.ppu.headless = false;

    gameboy.run_frame();

    assert_eq!(gameboy.frames, 1);
    assert_eq!(gameboy.mem[0x8000], 0xFF);
    assert_eq!(gameboy.mem[0x8001], 0x00);
    assert_eq!(gameboy.mem[0x9800], 0x00);
    assert_eq!(gameboy.ppu.framebuffer[0], 0x00081820);
}

#[test]
fn mbc1_switches_the_bank_mapped_at_0x4000() {
    let mut rom = vec![0; 0x10000];
    rom[0x147] = 0x01;
    rom[0x148] = 0x01;
    rom[0x4000] = 0x11;
    rom[0x8000] = 0x22;

    let mut gameboy = Gameboy::load(&rom);

    assert_eq!(gameboy.read_u8_addr(0x4000), 0x11);

    gameboy.write_u8_addr(0x2000, 0x02);

    assert_eq!(gameboy.read_u8_addr(0x4000), 0x22);
    assert_eq!(gameboy.mem[0x4000], 0x22);
}
