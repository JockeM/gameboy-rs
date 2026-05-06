use std::env;
use std::fs;
use std::process::ExitCode;

use agent::NoopAgent;
use train::{run_episode, EpisodeConfig};

fn main() -> ExitCode {
    let Some(rom_path) = env::args().nth(1) else {
        eprintln!("Usage: cargo run -p train --bin run_episode -- path/to/tetris.gb");
        return ExitCode::from(2);
    };

    let rom = match fs::read(&rom_path) {
        Ok(rom) => rom,
        Err(err) => {
            eprintln!("Failed to read ROM {rom_path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut agent = NoopAgent;
    let summary = run_episode(rom, &mut agent, &EpisodeConfig::default());

    println!("steps: {}", summary.steps);
    println!("total_reward: {}", summary.total_reward);
    println!("done: {}", summary.done);

    ExitCode::SUCCESS
}
