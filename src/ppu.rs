pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

const LIGHTEST: u32 = 0x00E0F8D0;
const LIGHT: u32 = 0x0088C070;
const DARK: u32 = 0x00346856;
const DARKEST: u32 = 0x00081820;

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
                let color = match tile {
                    0 => LIGHTEST,
                    1 => LIGHT,
                    2 => DARK,
                    _ => DARKEST,
                };

                self.framebuffer[y * SCREEN_WIDTH + x] = color;
            }
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}
