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
