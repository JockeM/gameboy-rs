use std::env as std_env;
use std::fs;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use env::{BOARD_WIDTH, TetrisMemoryMap, TetrisState};
use gameboy_rs::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gameboy_rs::{Gameboy, Input};
use minifb::{Key, Scale, Window, WindowOptions};

const CPU_CLOCK_HZ: u64 = 4_194_304;
const CYCLES_PER_FRAME: u64 = 70_224;
const FRAME_DURATION: Duration =
    Duration::from_nanos(CYCLES_PER_FRAME * 1_000_000_000 / CPU_CLOCK_HZ);
const DEFAULT_LOG_EVERY: u64 = 60;

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

struct CliConfig {
    rom_path: String,
    log_every: u64,
}

fn main() -> ExitCode {
    let config = match CliConfig::from_args(std_env::args().skip(1)) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "Usage: cargo run -p viewer --bin tetris_debug -- path/to/tetris.gb [--log-every N]"
            );
            return ExitCode::from(2);
        }
    };

    let rom = match fs::read(&config.rom_path) {
        Ok(rom) => rom,
        Err(err) => {
            eprintln!("Failed to read ROM {}: {err}", config.rom_path);
            return ExitCode::FAILURE;
        }
    };

    let mut gameboy = Gameboy::load(&rom);
    gameboy.ppu.headless = false;

    match run(&mut gameboy, &config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Viewer error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(gameboy: &mut Gameboy, config: &CliConfig) -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "gameboy-rs tetris debug",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    )?;

    println!("rom: {}", config.rom_path);
    println!("log_every: {}", config.log_every);
    println!("press Escape to quit");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        update_input(&window, gameboy);
        gameboy.run_frame();

        if gameboy.frames == 1 || gameboy.frames % config.log_every == 0 {
            let state = TetrisState::from_memory(
                gameboy.mem.as_slice(),
                gameboy.frames,
                TetrisMemoryMap::GAME_BOY_TETRIS,
            );
            print_state(&state);
        }

        window.update_with_buffer(&gameboy.ppu.framebuffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}

fn update_input(window: &Window, gameboy: &mut Gameboy) {
    let input = KEY_BINDINGS
        .iter()
        .filter_map(|(key, input)| window.is_key_down(*key).then_some(*input))
        .fold(Input::empty(), |input, pressed| input | pressed);

    gameboy.set_input(input);
}

fn print_state(state: &TetrisState) {
    println!(
        "frame={} score={} lines={} level={} occupied={} current={:?} preview={:?} next_preview={:?} center={:?}",
        state.frame,
        state.score,
        state.lines,
        state.level,
        occupied_count(state),
        state.pieces.map(|pieces| pieces.current),
        state.pieces.map(|pieces| pieces.preview),
        state.pieces.map(|pieces| pieces.next_preview),
        state.pieces.map(|pieces| pieces.center),
    );

    for row in state.occupied {
        let mut line = String::with_capacity(BOARD_WIDTH);
        for occupied in row {
            line.push(if occupied { '#' } else { '.' });
        }
        println!("{line}");
    }
    println!();
}

fn occupied_count(state: &TetrisState) -> usize {
    state
        .occupied
        .iter()
        .flatten()
        .filter(|occupied| **occupied)
        .count()
}

impl CliConfig {
    fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let rom_path = args.next().ok_or("missing ROM path")?;
        let mut log_every = DEFAULT_LOG_EVERY;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--log-every" => {
                    log_every = parse_u64_arg("--log-every", args.next())?;
                    if log_every == 0 {
                        return Err("--log-every must be greater than 0".to_string());
                    }
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }

        Ok(Self {
            rom_path,
            log_every,
        })
    }
}

fn parse_u64_arg(name: &str, value: Option<String>) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;

    value
        .parse()
        .map_err(|_| format!("invalid integer for {name}: {value}"))
}
