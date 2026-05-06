use crate::gameboy::{Action, EnvConfig, GameboyEnv, Observation};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TetrisAction {
    Noop,
    Left,
    Right,
    Down,
    RotateClockwise,
    RotateCounterClockwise,
    Start,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TetrisObservation {
    pub gameboy: Observation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TetrisStep {
    pub observation: TetrisObservation,
    pub reward: f32,
    pub done: bool,
}

pub struct TetrisEnv {
    gameboy: GameboyEnv,
}

impl TetrisEnv {
    pub fn new(rom: Vec<u8>, config: EnvConfig) -> Self {
        Self {
            gameboy: GameboyEnv::new(rom, config),
        }
    }

    pub fn reset(&mut self) -> TetrisObservation {
        TetrisObservation {
            gameboy: self.gameboy.reset(),
        }
    }

    pub fn step(&mut self, action: TetrisAction) -> TetrisStep {
        let step = self.gameboy.step(action.gameboy_action());

        TetrisStep {
            observation: TetrisObservation {
                gameboy: step.observation,
            },
            reward: step.reward,
            done: step.done,
        }
    }

    pub fn gameboy(&self) -> &GameboyEnv {
        &self.gameboy
    }

    pub fn gameboy_mut(&mut self) -> &mut GameboyEnv {
        &mut self.gameboy
    }
}

impl TetrisAction {
    fn gameboy_action(self) -> Action {
        match self {
            Self::Noop => Action::Noop,
            Self::Left => Action::Left,
            Self::Right => Action::Right,
            Self::Down => Action::Down,
            Self::RotateClockwise => Action::A,
            Self::RotateCounterClockwise => Action::B,
            Self::Start => Action::Start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tetris_step_wraps_gameboy_observation() {
        let mut env = TetrisEnv::new(
            test_rom(),
            EnvConfig {
                action_frames: 2,
                render_pixels: false,
            },
        );

        let step = env.step(TetrisAction::RotateClockwise);

        assert_eq!(step.observation.gameboy.frame, 2);
        assert_eq!(step.reward, 0.0);
        assert!(!step.done);
    }

    fn test_rom() -> Vec<u8> {
        let mut rom = vec![0; 0x150];
        rom[0x100] = 0x00;
        rom[0x101] = 0x18;
        rom[0x102] = 0xFD;
        rom
    }
}
