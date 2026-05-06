use gameboy_rs::ppu::{Ppu, CYCLES_PER_SCANLINE, SCREEN_HEIGHT, SCREEN_WIDTH};

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

#[test]
fn steps_through_scanline_modes_and_renders_on_hblank() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0x93;
    memory[0xFF47] = 0xE4;
    memory[0xFF48] = 0x0C;
    memory[0x8000] = 0x80;
    memory[0x8001] = 0x00;
    memory[0x8010] = 0x80;
    memory[0x8011] = 0x00;
    memory[0x9800] = 0x00;
    memory[0xFE00] = 16;
    memory[0xFE01] = 8;
    memory[0xFE02] = 1;
    ppu.sync_registers(&mut memory);

    ppu.step(&mut memory, 80);
    assert_eq!(memory[0xFF41] & 0x03, 0x03);

    ppu.step(&mut memory, 172);
    assert_eq!(memory[0xFF41] & 0x03, 0x00);
    assert_eq!(ppu.framebuffer[0], 0x00081820);
    assert_eq!(ppu.framebuffer[1], 0x00E0F8D0);
}

#[test]
fn requests_stat_interrupt_on_lyc_match() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0x91;
    memory[0xFF41] = 0x40;
    memory[0xFF45] = 1;
    ppu.sync_registers(&mut memory);

    ppu.step(&mut memory, u64::from(CYCLES_PER_SCANLINE));

    assert_eq!(memory[0xFF44], 1);
    assert_eq!(memory[0xFF41] & 0x04, 0x04);
    assert_eq!(memory[0xFF0F] & 0x02, 0x02);
}

#[test]
fn sprite_priority_keeps_non_zero_background_pixel_visible() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0x93;
    memory[0xFF47] = 0xE4;
    memory[0xFF48] = 0x0C;
    memory[0x8000] = 0x80;
    memory[0x8001] = 0x00;
    memory[0x8010] = 0x80;
    memory[0x8011] = 0x00;
    memory[0x9800] = 0x00;
    memory[0xFE00] = 16;
    memory[0xFE01] = 8;
    memory[0xFE02] = 1;
    memory[0xFE03] = 0x80;

    ppu.render_background(&memory);

    assert_eq!(ppu.framebuffer[0], 0x0088C070);
}

#[test]
fn renders_8x16_sprite_with_flips_and_second_palette() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0x97;
    memory[0xFF49] = 0x0C;
    memory[0x9800] = 0x00;
    memory[0x803E] = 0x01;
    memory[0x803F] = 0x00;
    memory[0xFE00] = 16;
    memory[0xFE01] = 8;
    memory[0xFE02] = 0x02;
    memory[0xFE03] = 0x70;

    ppu.render_background(&memory);

    assert_eq!(ppu.framebuffer[0], 0x00081820);
    assert_eq!(ppu.framebuffer[1], 0x00E0F8D0);
}

#[test]
fn window_overlays_background_from_wx_wy_position() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0xF1;
    memory[0xFF47] = 0xFC;
    memory[0xFF4A] = 0;
    memory[0xFF4B] = 7;
    memory[0x8000] = 0x00;
    memory[0x8001] = 0x00;
    memory[0x8010] = 0x80;
    memory[0x8011] = 0x00;
    memory[0x9800] = 0x00;
    memory[0x9C00] = 0x01;

    ppu.render_background(&memory);

    assert_eq!(ppu.framebuffer[0], 0x00081820);
    assert_eq!(ppu.framebuffer[1], 0x00E0F8D0);
}

#[test]
fn window_respects_wy_before_becoming_visible() {
    let mut ppu = Ppu::new();
    let mut memory = [0; 0x10000];
    memory[0xFF40] = 0xF1;
    memory[0xFF47] = 0xFC;
    memory[0xFF4A] = 1;
    memory[0xFF4B] = 7;
    memory[0x8010] = 0x80;
    memory[0x8011] = 0x00;
    memory[0x9C00] = 0x01;

    ppu.render_background(&memory);

    assert_eq!(ppu.framebuffer[0], 0x00E0F8D0);
    assert_eq!(ppu.framebuffer[SCREEN_WIDTH], 0x00081820);
}
