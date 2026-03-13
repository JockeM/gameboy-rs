pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

const LIGHTEST: u32 = 0x00E0F8D0;
const LIGHT: u32 = 0x0088C070;
const DARK: u32 = 0x00346856;
const DARKEST: u32 = 0x00081820;
const COLORS: [u32; 4] = [LIGHTEST, LIGHT, DARK, DARKEST];

pub struct Ppu {
    pub framebuffer: [u32; SCREEN_WIDTH * SCREEN_HEIGHT],
}

impl Ppu {
    pub fn new() -> Self {
        let mut ppu = Self {
            framebuffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
        };
        ppu.render_checkerboard();
        ppu
    }

    pub fn render_checkerboard(&mut self) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let tile = ((x / 8) + (y / 8)) % 4;
                let color = COLORS[tile];

                self.framebuffer[y * SCREEN_WIDTH + x] = color;
            }
        }
    }

    pub fn render_background(&mut self, memory: &[u8; 0x10000]) {
        let lcdc = memory[0xFF40];

        if lcdc & 0x80 == 0 {
            self.framebuffer.fill(COLORS[0]);
            return;
        }

        let bg_enabled = lcdc & 0x01 != 0;
        if !bg_enabled {
            self.framebuffer.fill(COLORS[0]);
            return;
        }

        let scroll_y = memory[0xFF42];
        let scroll_x = memory[0xFF43];
        let tile_map_base = if lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 };
        let unsigned_tile_data = lcdc & 0x10 != 0;
        let palette = memory[0xFF47];

        for y in 0..SCREEN_HEIGHT {
            let bg_y = scroll_y.wrapping_add(y as u8);
            let tile_y = usize::from(bg_y / 8);
            let row_in_tile = usize::from(bg_y % 8);

            for x in 0..SCREEN_WIDTH {
                let bg_x = scroll_x.wrapping_add(x as u8);
                let tile_x = usize::from(bg_x / 8);
                let col_in_tile = usize::from(bg_x % 8);
                let tile_index_address = tile_map_base + tile_y * 32 + tile_x;
                let tile_index = memory[tile_index_address];
                let tile_address = tile_data_address(tile_index, unsigned_tile_data);
                let lo = memory[tile_address + row_in_tile * 2];
                let hi = memory[tile_address + row_in_tile * 2 + 1];
                let bit = 7 - col_in_tile;
                let color_index = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                let shade = (palette >> (color_index * 2)) & 0b11;

                self.framebuffer[y * SCREEN_WIDTH + x] = COLORS[usize::from(shade)];
            }
        }
    }
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
