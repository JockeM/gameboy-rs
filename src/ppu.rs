pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const VISIBLE_SCANLINES: u8 = 144;
pub const TOTAL_SCANLINES: u8 = 154;
pub const CYCLES_PER_SCANLINE: u16 = 456;

const LCDC_ADDR: usize = 0xFF40;
const STAT_ADDR: usize = 0xFF41;
const SCROLL_Y_ADDR: usize = 0xFF42;
const SCROLL_X_ADDR: usize = 0xFF43;
const LY_ADDR: usize = 0xFF44;
const LYC_ADDR: usize = 0xFF45;
const BG_PALETTE_ADDR: usize = 0xFF47;
const OBJECT_PALETTE_0_ADDR: usize = 0xFF48;
const OBJECT_PALETTE_1_ADDR: usize = 0xFF49;
const WINDOW_Y_ADDR: usize = 0xFF4A;
const WINDOW_X_ADDR: usize = 0xFF4B;
const INTERRUPT_FLAG_ADDR: usize = 0xFF0F;
const OAM_START_ADDR: usize = 0xFE00;

const LIGHTEST: u32 = 0x00E0F8D0;
const LIGHT: u32 = 0x0088C070;
const DARK: u32 = 0x00346856;
const DARKEST: u32 = 0x00081820;
const COLORS: [u32; 4] = [LIGHTEST, LIGHT, DARK, DARKEST];

const MODE_HBLANK: u8 = 0;
const MODE_VBLANK: u8 = 1;
const MODE_OAM_SCAN: u8 = 2;
const MODE_DRAWING: u8 = 3;

const MODE_2_CYCLES: u16 = 80;
const MODE_3_CYCLES: u16 = 172;
const MODE_0_CYCLES: u16 = 204;

const SPRITE_PRIORITY_FLAG: u8 = 0x80;
const SPRITE_Y_FLIP_FLAG: u8 = 0x40;
const SPRITE_X_FLIP_FLAG: u8 = 0x20;
const SPRITE_PALETTE_FLAG: u8 = 0x10;

pub struct Ppu {
    pub framebuffer: [u32; SCREEN_WIDTH * SCREEN_HEIGHT],
    scanline_cycles: u16,
    mode: u8,
}

impl Ppu {
    pub fn new() -> Self {
        let mut ppu = Self {
            framebuffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            scanline_cycles: 0,
            mode: MODE_OAM_SCAN,
        };
        ppu.render_checkerboard();
        ppu
    }

    pub fn sync_registers(&mut self, memory: &mut [u8; 0x10000]) {
        if lcd_enabled(memory) {
            self.mode = MODE_OAM_SCAN;
            self.scanline_cycles = 0;
            self.set_ly(memory, 0);
            self.set_mode(memory, MODE_OAM_SCAN);
        } else {
            self.disable_lcd(memory);
        }
    }

    pub fn write_lcdc(&mut self, memory: &mut [u8; 0x10000], value: u8) {
        let was_enabled = lcd_enabled(memory);
        memory[LCDC_ADDR] = value;

        if value & 0x80 == 0 {
            self.disable_lcd(memory);
        } else if !was_enabled {
            self.mode = MODE_OAM_SCAN;
            self.scanline_cycles = 0;
            self.set_ly(memory, 0);
            self.set_mode(memory, MODE_OAM_SCAN);
        }
    }

    pub fn write_stat(&mut self, memory: &mut [u8; 0x10000], value: u8) {
        memory[STAT_ADDR] = (value & 0xF8) | (memory[STAT_ADDR] & 0x07);
        self.update_coincidence_flag(memory);
    }

    pub fn reset_ly(&mut self, memory: &mut [u8; 0x10000]) {
        self.set_ly(memory, 0);
    }

    pub fn render_checkerboard(&mut self) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let tile = ((x / 8) + (y / 8)) % 4;
                self.framebuffer[y * SCREEN_WIDTH + x] = COLORS[tile];
            }
        }
    }

    pub fn render_background(&mut self, memory: &[u8; 0x10000]) {
        if !lcd_enabled(memory) || memory[LCDC_ADDR] & 0x01 == 0 {
            self.framebuffer.fill(COLORS[0]);
            return;
        }

        for scanline in 0..SCREEN_HEIGHT {
            self.render_scanline(memory, scanline);
        }
    }

    pub fn step(&mut self, memory: &mut [u8; 0x10000], elapsed_cycles: u64) {
        if elapsed_cycles == 0 {
            return;
        }

        if !lcd_enabled(memory) {
            if self.mode != MODE_HBLANK || memory[LY_ADDR] != 0 {
                self.disable_lcd(memory);
            }
            return;
        }

        if elapsed_cycles <= u64::from(u16::MAX - self.scanline_cycles) {
            let next_scanline_cycles = self.scanline_cycles + elapsed_cycles as u16;
            if next_scanline_cycles < self.mode_cycles() {
                self.scanline_cycles = next_scanline_cycles;
                return;
            }
        }

        let mut remaining_cycles = elapsed_cycles;

        while remaining_cycles > 0 {
            let mode_cycles = u64::from(self.mode_cycles());
            let progressed_cycles = u64::from(self.scanline_cycles);
            let step_cycles = remaining_cycles.min(mode_cycles - progressed_cycles);

            self.scanline_cycles += step_cycles as u16;
            remaining_cycles -= step_cycles;

            if u64::from(self.scanline_cycles) < mode_cycles {
                continue;
            }

            self.scanline_cycles = 0;
            self.advance_mode(memory);
        }
    }

    fn advance_mode(&mut self, memory: &mut [u8; 0x10000]) {
        match self.mode {
            MODE_OAM_SCAN => self.set_mode(memory, MODE_DRAWING),
            MODE_DRAWING => {
                let scanline = memory[LY_ADDR];
                if scanline < VISIBLE_SCANLINES {
                    self.render_scanline(memory, usize::from(scanline));
                }
                self.set_mode(memory, MODE_HBLANK);
            }
            MODE_HBLANK => {
                let next_scanline = memory[LY_ADDR].wrapping_add(1);
                self.set_ly(memory, next_scanline);

                if next_scanline >= VISIBLE_SCANLINES {
                    self.mem_request_interrupt(memory, 0x01);
                    self.set_mode(memory, MODE_VBLANK);
                } else {
                    self.set_mode(memory, MODE_OAM_SCAN);
                }
            }
            MODE_VBLANK => {
                let next_scanline = if memory[LY_ADDR] + 1 >= TOTAL_SCANLINES {
                    0
                } else {
                    memory[LY_ADDR] + 1
                };

                self.set_ly(memory, next_scanline);

                if next_scanline == 0 {
                    self.set_mode(memory, MODE_OAM_SCAN);
                }
            }
            _ => unreachable!("invalid PPU mode {}", self.mode),
        }
    }

    fn mode_cycles(&self) -> u16 {
        match self.mode {
            MODE_HBLANK => MODE_0_CYCLES,
            MODE_VBLANK => CYCLES_PER_SCANLINE,
            MODE_OAM_SCAN => MODE_2_CYCLES,
            MODE_DRAWING => MODE_3_CYCLES,
            _ => unreachable!("invalid PPU mode {}", self.mode),
        }
    }

    fn disable_lcd(&mut self, memory: &mut [u8; 0x10000]) {
        self.scanline_cycles = 0;
        self.mode = MODE_HBLANK;
        self.framebuffer.fill(COLORS[0]);
        self.set_ly(memory, 0);
        self.set_mode(memory, MODE_HBLANK);
    }

    fn render_scanline(&mut self, memory: &[u8; 0x10000], y: usize) {
        let mut background_color_indices = [0; SCREEN_WIDTH];

        if memory[LCDC_ADDR] & 0x01 == 0 {
            let row = &mut self.framebuffer[y * SCREEN_WIDTH..(y + 1) * SCREEN_WIDTH];
            row.fill(COLORS[0]);
        } else {
            self.render_background_scanline(memory, y, &mut background_color_indices);
            self.render_window_scanline(memory, y, &mut background_color_indices);
        }

        if memory[LCDC_ADDR] & 0x02 != 0 {
            self.render_sprite_scanline(memory, y, &background_color_indices);
        }
    }

    fn render_background_scanline(
        &mut self,
        memory: &[u8; 0x10000],
        y: usize,
        background_color_indices: &mut [u8; SCREEN_WIDTH],
    ) {
        let scroll_y = memory[SCROLL_Y_ADDR];
        let scroll_x = memory[SCROLL_X_ADDR];
        let tile_map_base = if memory[LCDC_ADDR] & 0x08 != 0 {
            0x9C00
        } else {
            0x9800
        };
        let unsigned_tile_data = memory[LCDC_ADDR] & 0x10 != 0;
        let palette = memory[BG_PALETTE_ADDR];
        let bg_y = scroll_y.wrapping_add(y as u8);
        let tile_y = usize::from(bg_y / 8);
        let row_in_tile = usize::from(bg_y % 8);

        let mut x = 0;
        while x < SCREEN_WIDTH {
            let bg_x = scroll_x.wrapping_add(x as u8);
            let tile_x = usize::from(bg_x >> 3);
            let col_in_tile = usize::from(bg_x & 0x07);
            let tile_index_address = tile_map_base + tile_y * 32 + tile_x;
            let tile_index = memory[tile_index_address];
            let tile_address = tile_data_address(tile_index, unsigned_tile_data);
            let lo = memory[tile_address + row_in_tile * 2];
            let hi = memory[tile_address + row_in_tile * 2 + 1];
            let pixels = (8 - col_in_tile).min(SCREEN_WIDTH - x);

            for pixel in 0..pixels {
                let bit = 7 - (col_in_tile + pixel);
                let color_index = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                let shade = palette_shade(palette, color_index);
                let screen_x = x + pixel;

                background_color_indices[screen_x] = color_index;
                self.framebuffer[y * SCREEN_WIDTH + screen_x] = COLORS[usize::from(shade)];
            }

            x += pixels;
        }
    }

    fn render_sprite_scanline(
        &mut self,
        memory: &[u8; 0x10000],
        y: usize,
        background_color_indices: &[u8; SCREEN_WIDTH],
    ) {
        let sprite_height = sprite_height(memory[LCDC_ADDR]);
        let mut visible_sprites = [VisibleSprite::default(); 10];
        let mut visible_sprite_count = 0usize;

        for sprite_index in 0..40usize {
            let oam_addr = OAM_START_ADDR + sprite_index * 4;
            let sprite_y = i16::from(memory[oam_addr]) - 16;
            let sprite_x = i16::from(memory[oam_addr + 1]) - 8;

            if !sprite_covers_scanline(sprite_y, sprite_height, y) {
                continue;
            }

            visible_sprites[visible_sprite_count] = VisibleSprite {
                x: memory[oam_addr + 1],
                oam_index: sprite_index as u8,
                screen_x: sprite_x,
                screen_y: sprite_y,
                tile_index: memory[oam_addr + 2],
                attributes: memory[oam_addr + 3],
            };
            visible_sprite_count += 1;

            if visible_sprite_count == visible_sprites.len() {
                break;
            }
        }

        let visible_sprites = &mut visible_sprites[..visible_sprite_count];
        visible_sprites.sort_by_key(|sprite| (sprite.x, sprite.oam_index));

        for sprite in visible_sprites.iter().rev() {
            let mut row_in_sprite = (y as i16 - sprite.screen_y) as usize;
            if sprite.attributes & SPRITE_Y_FLIP_FLAG != 0 {
                row_in_sprite = sprite_height - 1 - row_in_sprite;
            }

            let tile_index = if sprite_height == 16 {
                sprite.tile_index & 0xFE
            } else {
                sprite.tile_index
            };
            let tile_address = if row_in_sprite >= 8 {
                0x8000 + usize::from(tile_index.wrapping_add(1)) * 16
            } else {
                0x8000 + usize::from(tile_index) * 16
            };
            let tile_row = row_in_sprite % 8;
            let lo = memory[tile_address + tile_row * 2];
            let hi = memory[tile_address + tile_row * 2 + 1];
            let palette = if sprite.attributes & SPRITE_PALETTE_FLAG != 0 {
                memory[OBJECT_PALETTE_1_ADDR]
            } else {
                memory[OBJECT_PALETTE_0_ADDR]
            };

            for x in 0..8usize {
                let screen_x = sprite.screen_x + x as i16;
                if !(0..SCREEN_WIDTH as i16).contains(&screen_x) {
                    continue;
                }

                let bit = if sprite.attributes & SPRITE_X_FLIP_FLAG != 0 {
                    x as u8
                } else {
                    7 - x as u8
                };
                let color_index = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                if color_index == 0 {
                    continue;
                }

                let screen_x = screen_x as usize;
                if sprite.attributes & SPRITE_PRIORITY_FLAG != 0
                    && background_color_indices[screen_x] != 0
                {
                    continue;
                }

                let shade = palette_shade(palette, color_index);
                self.framebuffer[y * SCREEN_WIDTH + screen_x] = COLORS[usize::from(shade)];
            }
        }
    }

    fn render_window_scanline(
        &mut self,
        memory: &[u8; 0x10000],
        y: usize,
        background_color_indices: &mut [u8; SCREEN_WIDTH],
    ) {
        if memory[LCDC_ADDR] & 0x20 == 0 {
            return;
        }

        let window_y = usize::from(memory[WINDOW_Y_ADDR]);
        if y < window_y {
            return;
        }

        let window_x = i16::from(memory[WINDOW_X_ADDR]) - 7;
        let tile_map_base = if memory[LCDC_ADDR] & 0x40 != 0 {
            0x9C00
        } else {
            0x9800
        };
        let unsigned_tile_data = memory[LCDC_ADDR] & 0x10 != 0;
        let palette = memory[BG_PALETTE_ADDR];
        let window_line = y - window_y;
        let tile_y = window_line / 8;
        let row_in_tile = window_line % 8;
        let start_x = window_x.max(0) as usize;

        let mut x = start_x;
        while x < SCREEN_WIDTH {
            let window_col = (x as i16 - window_x) as usize;
            let tile_x = window_col >> 3;
            let col_in_tile = window_col & 0x07;
            let tile_index_address = tile_map_base + tile_y * 32 + tile_x;
            let tile_index = memory[tile_index_address];
            let tile_address = tile_data_address(tile_index, unsigned_tile_data);
            let lo = memory[tile_address + row_in_tile * 2];
            let hi = memory[tile_address + row_in_tile * 2 + 1];
            let pixels = (8 - col_in_tile).min(SCREEN_WIDTH - x);

            for pixel in 0..pixels {
                let bit = 7 - (col_in_tile + pixel);
                let color_index = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                let shade = palette_shade(palette, color_index);
                let screen_x = x + pixel;

                background_color_indices[screen_x] = color_index;
                self.framebuffer[y * SCREEN_WIDTH + screen_x] = COLORS[usize::from(shade)];
            }

            x += pixels;
        }
    }

    fn set_ly(&mut self, memory: &mut [u8; 0x10000], value: u8) {
        memory[LY_ADDR] = value;
        self.update_coincidence_flag(memory);
    }

    fn set_mode(&mut self, memory: &mut [u8; 0x10000], mode: u8) {
        self.mode = mode;
        memory[STAT_ADDR] = (memory[STAT_ADDR] & 0xFC) | mode;

        let stat_interrupt = match mode {
            MODE_HBLANK => memory[STAT_ADDR] & 0x08 != 0,
            MODE_VBLANK => memory[STAT_ADDR] & 0x10 != 0,
            MODE_OAM_SCAN => memory[STAT_ADDR] & 0x20 != 0,
            MODE_DRAWING => false,
            _ => unreachable!("invalid PPU mode {}", mode),
        };

        if stat_interrupt {
            self.mem_request_interrupt(memory, 0x02);
        }
    }

    fn update_coincidence_flag(&mut self, memory: &mut [u8; 0x10000]) {
        let had_coincidence = memory[STAT_ADDR] & 0x04 != 0;
        let has_coincidence = memory[LY_ADDR] == memory[LYC_ADDR];

        if has_coincidence {
            memory[STAT_ADDR] |= 0x04;
        } else {
            memory[STAT_ADDR] &= !0x04;
        }

        if !had_coincidence && has_coincidence && memory[STAT_ADDR] & 0x40 != 0 {
            self.mem_request_interrupt(memory, 0x02);
        }
    }

    fn mem_request_interrupt(&self, memory: &mut [u8; 0x10000], bit: u8) {
        memory[INTERRUPT_FLAG_ADDR] |= bit;
    }
}

fn lcd_enabled(memory: &[u8; 0x10000]) -> bool {
    memory[LCDC_ADDR] & 0x80 != 0
}

fn tile_data_address(tile_index: u8, unsigned_tile_data: bool) -> usize {
    if unsigned_tile_data {
        0x8000 + usize::from(tile_index) * 16
    } else {
        (0x9000i32 + i32::from(tile_index as i8) * 16) as usize
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Default)]
struct VisibleSprite {
    x: u8,
    oam_index: u8,
    screen_x: i16,
    screen_y: i16,
    tile_index: u8,
    attributes: u8,
}

fn sprite_height(lcdc: u8) -> usize {
    if lcdc & 0x04 != 0 {
        16
    } else {
        8
    }
}

fn sprite_covers_scanline(sprite_y: i16, sprite_height: usize, scanline: usize) -> bool {
    let scanline = scanline as i16;
    scanline >= sprite_y && scanline < sprite_y + sprite_height as i16
}

fn palette_shade(palette: u8, color_index: u8) -> u8 {
    (palette >> (color_index * 2)) & 0b11
}
