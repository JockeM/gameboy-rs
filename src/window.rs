use std::thread;
use std::time::{Duration, Instant};

use crate::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::Gameboy;

use minifb::{Key, Scale, Window, WindowOptions};

const CPU_CLOCK_HZ: u64 = 4_194_304;
const CYCLES_PER_FRAME: u64 = 70_224;
const FRAME_DURATION: Duration =
    Duration::from_nanos(CYCLES_PER_FRAME * 1_000_000_000 / CPU_CLOCK_HZ);

pub fn run(gameboy: &mut Gameboy) -> Result<(), minifb::Error> {
    gameboy.ppu.headless = false;

    let mut window = Window::new(
        "gameboy-rs",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    )?;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        update_joypad(&window, gameboy);
        gameboy.run_frame();
        window.update_with_buffer(&gameboy.ppu.framebuffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}

fn update_joypad(window: &Window, gameboy: &mut Gameboy) {
    let mut directions = 0x0F;
    let mut buttons = 0x0F;

    if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
        directions &= !0x01;
    }
    if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
        directions &= !0x02;
    }
    if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
        directions &= !0x04;
    }
    if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
        directions &= !0x08;
    }
    if window.is_key_down(Key::Z) || window.is_key_down(Key::J) {
        buttons &= !0x01;
    }
    if window.is_key_down(Key::X) || window.is_key_down(Key::K) {
        buttons &= !0x02;
    }
    if window.is_key_down(Key::Backspace) || window.is_key_down(Key::RightShift) {
        buttons &= !0x04;
    }
    if window.is_key_down(Key::Enter) || window.is_key_down(Key::Space) {
        buttons &= !0x08;
    }

    gameboy.set_joypad_state(buttons, directions);
}
