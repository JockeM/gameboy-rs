use agent::Agent;
use env::{EnvConfig, TetrisEnv, TetrisMemoryMap};

pub mod dqn;
pub mod dqn_agent;
pub mod model;
pub mod replay;
pub mod rng;

#[derive(Clone, Debug)]
pub struct EpisodeConfig {
    pub max_steps: usize,
    pub env: EnvConfig,
}

impl Default for EpisodeConfig {
    fn default() -> Self {
        Self {
            max_steps: 1_000,
            env: EnvConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpisodeSummary {
    pub steps: usize,
    pub total_reward: f32,
    pub done: bool,
}

pub fn run_episode(rom: Vec<u8>, agent: &mut impl Agent, config: &EpisodeConfig) -> EpisodeSummary {
    let mut env =
        TetrisEnv::new_with_memory_map(rom, config.env.clone(), TetrisMemoryMap::GAME_BOY_TETRIS);
    let mut observation = env.reset();
    let mut total_reward = 0.0;

    for step_index in 0..config.max_steps {
        let action = agent.act(&observation);
        let step = env.step(action);

        total_reward += step.reward;
        observation = step.observation;

        if step.done {
            return EpisodeSummary {
                steps: step_index + 1,
                total_reward,
                done: true,
            };
        }
    }

    EpisodeSummary {
        steps: config.max_steps,
        total_reward,
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent::NoopAgent;

    #[test]
    fn episode_runner_steps_until_limit() {
        let mut agent = NoopAgent;
        let config = EpisodeConfig {
            max_steps: 4,
            env: EnvConfig {
                action_frames: 2,
                render_pixels: false,
            },
        };

        let summary = run_episode(test_rom(), &mut agent, &config);

        assert_eq!(
            summary,
            EpisodeSummary {
                steps: 4,
                total_reward: 0.0,
                done: false,
            }
        );
    }

    fn test_rom() -> Vec<u8> {
        let mut rom = vec![0; 0x150];
        rom[0x100] = 0x00;
        rom[0x101] = 0x18;
        rom[0x102] = 0xFD;
        rom
    }
}
