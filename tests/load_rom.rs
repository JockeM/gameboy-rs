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
    assert_eq!(gameboy.mem[0x0000], 0x00);
    assert_eq!(gameboy.mem[0x0100], 0x00);
    assert_eq!(&gameboy.mem[0x0134..0x013a], b"SIMPLE");
}
