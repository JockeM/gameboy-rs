use std::thread;
use std::time::{Duration, Instant};

use crate::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::{Gameboy, Input};

use minifb::{Key, Scale, Window, WindowOptions};

const CPU_CLOCK_HZ: u64 = 4_194_304;
const CYCLES_PER_FRAME: u64 = 70_224;
const FRAME_DURATION: Duration =
    Duration::from_nanos(CYCLES_PER_FRAME * 1_000_000_000 / CPU_CLOCK_HZ);
const KEY_BINDINGS: &[(Key, Input)] = &[
    (Key::Right, Input::RIGHT),
    (Key::D, Input::RIGHT),
    (Key::Left, Input::LEFT),
    (Key::A, Input::LEFT),
    (Key::Up, Input::UP),
    (Key::W, Input::UP),
    (Key::Down, Input::DOWN),
    (Key::S, Input::DOWN),
    (Key::Z, Input::A),
    (Key::J, Input::A),
    (Key::X, Input::B),
    (Key::K, Input::B),
    (Key::Backspace, Input::SELECT),
    (Key::RightShift, Input::SELECT),
    (Key::Enter, Input::START),
    (Key::Space, Input::START),
];

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
    let input = KEY_BINDINGS
        .iter()
        .filter_map(|(key, input)| window.is_key_down(*key).then_some(*input))
        .fold(Input::empty(), |input, pressed| input | pressed);

    gameboy.set_input(input);
}
