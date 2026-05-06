use std::env as std_env;
use std::fs;
use std::process::ExitCode;

use agent::{Agent, NoopAgent};
use env::{EnvConfig, TetrisEnv};

const DEFAULT_MAX_STEPS: usize = 1_000;
const DEFAULT_LOG_EVERY: usize = 100;

struct CliConfig {
    rom_path: String,
    max_steps: usize,
    log_every: usize,
}

fn main() -> ExitCode {
    let config = match CliConfig::from_args(std_env::args().skip(1)) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "Usage: cargo run -p train --bin run_episode -- path/to/tetris.gb [--max-steps N] [--log-every N]"
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

    let mut agent = NoopAgent;
    let mut env = TetrisEnv::new(rom, EnvConfig::default());
    let mut observation = env.reset();
    let mut total_reward = 0.0;
    let mut steps = 0;
    let mut done = false;

    println!("rom: {}", config.rom_path);
    println!("max_steps: {}", config.max_steps);
    println!("log_every: {}", config.log_every);
    println!("starting episode");

    for step_index in 0..config.max_steps {
        let action = agent.act(&observation);
        let step = env.step(action);

        steps = step_index + 1;
        total_reward += step.reward;
        done = step.done;
        observation = step.observation;

        if steps == 1 || steps % config.log_every == 0 || done {
            println!(
                "step: {steps} frame: {} action: {action:?} reward: {} total_reward: {total_reward} done: {done}",
                observation.gameboy.frame, step.reward
            );
        }

        if done {
            break;
        }
    }

    println!("episode finished");
    println!("steps: {steps}");
    println!("total_reward: {total_reward}");
    println!("done: {done}");

    ExitCode::SUCCESS
}

impl CliConfig {
    fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let rom_path = args.next().ok_or("missing ROM path")?;
        let mut max_steps = DEFAULT_MAX_STEPS;
        let mut log_every = DEFAULT_LOG_EVERY;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--max-steps" => {
                    max_steps = parse_usize_arg("--max-steps", args.next())?;
                }
                "--log-every" => {
                    log_every = parse_usize_arg("--log-every", args.next())?;
                    if log_every == 0 {
                        return Err("--log-every must be greater than 0".to_string());
                    }
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }

        Ok(Self {
            rom_path,
            max_steps,
            log_every,
        })
    }
}

fn parse_usize_arg(name: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;

    value
        .parse()
        .map_err(|_| format!("invalid integer for {name}: {value}"))
}
