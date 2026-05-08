use std::env as std_env;
use std::fs;
use std::process::ExitCode;

use agent::{Agent, NoopAgent, StartThenNoopAgent};
use env::{EnvConfig, TetrisEnv, TetrisMemoryMap, TetrisState};

const DEFAULT_MAX_STEPS: usize = 1_000;
const DEFAULT_LOG_EVERY: usize = 100;
const DEFAULT_START_STEPS: usize = 30;

struct CliConfig {
    rom_path: String,
    max_steps: usize,
    log_every: usize,
    agent: AgentKind,
    start_steps: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AgentKind {
    Noop,
    StartThenNoop,
}

fn main() -> ExitCode {
    let config = match CliConfig::from_args(std_env::args().skip(1)) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "Usage: cargo run -p train --bin run_episode -- path/to/tetris.gb [--max-steps N] [--log-every N] [--agent noop|start-noop] [--start-steps N]"
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

    let mut agent = create_agent(&config);
    let mut env =
        TetrisEnv::new_with_memory_map(rom, EnvConfig::default(), TetrisMemoryMap::GAME_BOY_TETRIS);
    let mut observation = env.reset();
    let mut total_reward = 0.0;
    let mut steps = 0;
    let mut done = false;

    println!("rom: {}", config.rom_path);
    println!("max_steps: {}", config.max_steps);
    println!("log_every: {}", config.log_every);
    println!("agent: {:?}", config.agent);
    println!("start_steps: {}", config.start_steps);
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
            if let Some(state) = observation.state.as_ref() {
                print_tetris_state(state);
            }
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

fn create_agent(config: &CliConfig) -> Box<dyn Agent> {
    match config.agent {
        AgentKind::Noop => Box::new(NoopAgent),
        AgentKind::StartThenNoop => Box::new(StartThenNoopAgent::new(config.start_steps)),
    }
}

fn print_tetris_state(state: &TetrisState) {
    let features = state.board_features();

    println!(
        "score: {} lines: {} level: {} game_over: {} occupied: {} holes: {} height: {} bumpiness: {} current: {:?} preview: {:?}",
        state.score,
        state.lines,
        state.level,
        state.game_over,
        occupied_count(state),
        features.holes,
        features.aggregate_height,
        features.bumpiness,
        state.pieces.map(|pieces| pieces.current),
        state.pieces.map(|pieces| pieces.preview),
    );
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
        let mut max_steps = DEFAULT_MAX_STEPS;
        let mut log_every = DEFAULT_LOG_EVERY;
        let mut agent = AgentKind::StartThenNoop;
        let mut start_steps = DEFAULT_START_STEPS;

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
                "--agent" => {
                    agent = parse_agent_arg(args.next())?;
                }
                "--start-steps" => {
                    start_steps = parse_usize_arg("--start-steps", args.next())?;
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }

        Ok(Self {
            rom_path,
            max_steps,
            log_every,
            agent,
            start_steps,
        })
    }
}

fn parse_agent_arg(value: Option<String>) -> Result<AgentKind, String> {
    match value
        .ok_or_else(|| "missing value for --agent".to_string())?
        .as_str()
    {
        "noop" => Ok(AgentKind::Noop),
        "start-noop" => Ok(AgentKind::StartThenNoop),
        value => Err(format!("invalid agent: {value}")),
    }
}

fn parse_usize_arg(name: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;

    value
        .parse()
        .map_err(|_| format!("invalid integer for {name}: {value}"))
}
