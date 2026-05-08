use std::env as std_env;
use std::fs;
use std::process::ExitCode;

use agent::Agent;
use burn::backend::{Autodiff, NdArray};
use env::{EnvConfig, TetrisEnv, TetrisMemoryMap};
use train::dqn::{DEFAULT_DISCOUNT, DEFAULT_LEARNING_RATE, DqnTrainer};
use train::dqn_agent::BurnDqnAgent;
use train::model::{TetrisQNetwork, action_index, encode_observation};
use train::replay::{ReplayBuffer, Transition};
use train::rng::SimpleRng;

type Backend = Autodiff<NdArray<f32>>;

const DEFAULT_EPISODES: usize = 10;
const DEFAULT_MAX_STEPS: usize = 1_000;
const DEFAULT_LOG_EVERY: usize = 1;
const DEFAULT_REPLAY_CAPACITY: usize = 10_000;
const DEFAULT_BATCH_SIZE: usize = 32;
const DEFAULT_EPSILON: f32 = 0.1;
const DEFAULT_START_STEPS: usize = 30;
const DEFAULT_SEED: u64 = 0xD0D0_D0D0;

struct CliConfig {
    rom_path: String,
    episodes: usize,
    max_steps: usize,
    log_every: usize,
    replay_capacity: usize,
    batch_size: usize,
    epsilon: f32,
    start_steps: usize,
    seed: u64,
    learning_rate: f64,
    discount: f32,
}

fn main() -> ExitCode {
    let config = match CliConfig::from_args(std_env::args().skip(1)) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "Usage: cargo run -p train --bin train_dqn -- path/to/tetris.gb [--episodes N] [--max-steps N] [--log-every N] [--replay-capacity N] [--batch-size N] [--epsilon F] [--start-steps N] [--seed N] [--learning-rate F] [--discount F]"
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

    let device = Default::default();
    let model = TetrisQNetwork::<Backend>::new(&device);
    let mut agent = BurnDqnAgent::new(
        model,
        device,
        config.epsilon,
        config.seed,
        config.start_steps,
    );
    let mut replay = ReplayBuffer::new(config.replay_capacity);
    let mut sample_rng = SimpleRng::new(config.seed ^ 0xBAD5_EED);
    let mut trainer = DqnTrainer::<Backend>::new(config.learning_rate, config.discount);

    println!("rom: {}", config.rom_path);
    println!("episodes: {}", config.episodes);
    println!("max_steps: {}", config.max_steps);
    println!("replay_capacity: {}", config.replay_capacity);
    println!("batch_size: {}", config.batch_size);
    println!("epsilon: {}", config.epsilon);
    println!("start_steps: {}", config.start_steps);
    println!("seed: {}", config.seed);
    println!("learning_rate: {}", config.learning_rate);
    println!("discount: {}", config.discount);
    println!("starting DQN training");

    for episode in 1..=config.episodes {
        let summary = run_training_episode(
            &rom,
            &config,
            &mut agent,
            &mut replay,
            &mut sample_rng,
            &mut trainer,
        );

        if episode == 1 || episode % config.log_every == 0 {
            println!(
                "episode: {episode} steps: {} reward: {} done: {} replay: {}/{} updates: {} last_loss: {:?}",
                summary.steps,
                summary.total_reward,
                summary.done,
                replay.len(),
                replay.capacity(),
                summary.updates,
                summary.last_loss
            );
        }
    }

    println!("finished DQN training");
    println!("replay: {}/{}", replay.len(), replay.capacity());

    ExitCode::SUCCESS
}

struct TrainingEpisodeSummary {
    steps: usize,
    total_reward: f32,
    done: bool,
    updates: usize,
    last_loss: Option<f32>,
}

fn run_training_episode(
    rom: &[u8],
    config: &CliConfig,
    agent: &mut BurnDqnAgent<Backend>,
    replay: &mut ReplayBuffer,
    sample_rng: &mut SimpleRng,
    trainer: &mut DqnTrainer<Backend>,
) -> TrainingEpisodeSummary {
    let mut env = TetrisEnv::new_with_memory_map(
        rom.to_vec(),
        EnvConfig {
            action_frames: 6,
            render_pixels: false,
        },
        TetrisMemoryMap::GAME_BOY_TETRIS,
    );
    let mut observation = env.reset();
    let mut total_reward = 0.0;
    let mut updates = 0;
    let mut last_loss = None;

    for step_index in 0..config.max_steps {
        let state = encode_observation(&observation);
        let action = agent.act(&observation);
        let step = env.step(action);
        let next_state = encode_observation(&step.observation);

        replay.push(Transition {
            state,
            action: action_index(action),
            reward: step.reward,
            next_state,
            done: step.done,
        });

        if replay.len() >= config.batch_size {
            let batch = replay.sample(config.batch_size, sample_rng);
            let (model, stats) =
                trainer.train_step(agent.model().clone(), &batch, &Default::default());
            agent.set_model(model);
            updates += 1;
            last_loss = Some(stats.loss);
        }

        total_reward += step.reward;
        observation = step.observation;

        if step.done {
            return TrainingEpisodeSummary {
                steps: step_index + 1,
                total_reward,
                done: true,
                updates,
                last_loss,
            };
        }
    }

    TrainingEpisodeSummary {
        steps: config.max_steps,
        total_reward,
        done: false,
        updates,
        last_loss,
    }
}

impl CliConfig {
    fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let rom_path = args.next().ok_or("missing ROM path")?;
        let mut episodes = DEFAULT_EPISODES;
        let mut max_steps = DEFAULT_MAX_STEPS;
        let mut log_every = DEFAULT_LOG_EVERY;
        let mut replay_capacity = DEFAULT_REPLAY_CAPACITY;
        let mut batch_size = DEFAULT_BATCH_SIZE;
        let mut epsilon = DEFAULT_EPSILON;
        let mut start_steps = DEFAULT_START_STEPS;
        let mut seed = DEFAULT_SEED;
        let mut learning_rate = DEFAULT_LEARNING_RATE;
        let mut discount = DEFAULT_DISCOUNT;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--episodes" => {
                    episodes = parse_usize_arg("--episodes", args.next())?;
                }
                "--max-steps" => {
                    max_steps = parse_usize_arg("--max-steps", args.next())?;
                }
                "--log-every" => {
                    log_every = parse_usize_arg("--log-every", args.next())?;
                    if log_every == 0 {
                        return Err("--log-every must be greater than 0".to_string());
                    }
                }
                "--replay-capacity" => {
                    replay_capacity = parse_usize_arg("--replay-capacity", args.next())?;
                    if replay_capacity == 0 {
                        return Err("--replay-capacity must be greater than 0".to_string());
                    }
                }
                "--batch-size" => {
                    batch_size = parse_usize_arg("--batch-size", args.next())?;
                    if batch_size == 0 {
                        return Err("--batch-size must be greater than 0".to_string());
                    }
                }
                "--epsilon" => {
                    epsilon = parse_f32_arg("--epsilon", args.next())?;
                    if !(0.0..=1.0).contains(&epsilon) {
                        return Err("--epsilon must be between 0 and 1".to_string());
                    }
                }
                "--start-steps" => {
                    start_steps = parse_usize_arg("--start-steps", args.next())?;
                }
                "--seed" => {
                    seed = parse_u64_arg("--seed", args.next())?;
                }
                "--learning-rate" => {
                    learning_rate = parse_f64_arg("--learning-rate", args.next())?;
                    if learning_rate <= 0.0 {
                        return Err("--learning-rate must be greater than 0".to_string());
                    }
                }
                "--discount" => {
                    discount = parse_f32_arg("--discount", args.next())?;
                    if !(0.0..=1.0).contains(&discount) {
                        return Err("--discount must be between 0 and 1".to_string());
                    }
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }

        Ok(Self {
            rom_path,
            episodes,
            max_steps,
            log_every,
            replay_capacity,
            batch_size,
            epsilon,
            start_steps,
            seed,
            learning_rate,
            discount,
        })
    }
}

fn parse_usize_arg(name: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;
    value
        .parse()
        .map_err(|_| format!("invalid integer for {name}: {value}"))
}

fn parse_u64_arg(name: &str, value: Option<String>) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;
    value
        .parse()
        .map_err(|_| format!("invalid integer for {name}: {value}"))
}

fn parse_f32_arg(name: &str, value: Option<String>) -> Result<f32, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;
    value
        .parse()
        .map_err(|_| format!("invalid float for {name}: {value}"))
}

fn parse_f64_arg(name: &str, value: Option<String>) -> Result<f64, String> {
    let value = value.ok_or_else(|| format!("missing value for {name}"))?;
    value
        .parse()
        .map_err(|_| format!("invalid float for {name}: {value}"))
}
