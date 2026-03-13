use gameboy_rs::ppu::{Ppu, SCREEN_HEIGHT, SCREEN_WIDTH};

#[test]
fn initializes_a_full_screen_framebuffer() {
    let ppu = Ppu::new();

    assert_eq!(ppu.framebuffer.len(), SCREEN_WIDTH * SCREEN_HEIGHT);
    assert_ne!(ppu.framebuffer[0], 0);
}

#[test]
fn checkerboard_changes_color_by_tile() {
    let ppu = Ppu::new();

    assert_eq!(ppu.framebuffer[0], ppu.framebuffer[7]);
    assert_ne!(ppu.framebuffer[0], ppu.framebuffer[8]);
    assert_ne!(ppu.framebuffer[0], ppu.framebuffer[8 * SCREEN_WIDTH]);
}

#[test]
fn renders_background_tile_data_and_palette() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0x91;
    memory[0xFF47] = 0xFC;
    memory[0x8000] = 0x80;
    memory[0x8001] = 0x00;
    memory[0x9800] = 0x00;

    ppu.render_background(&memory);

    assert_eq!(ppu.framebuffer[0], 0x00081820);
    assert_eq!(ppu.framebuffer[1], 0x00E0F8D0);
}
