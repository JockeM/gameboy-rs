use gameboy_rs::{Gameboy, Input};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Noop,
    Left,
    Right,
    Up,
    Down,
    A,
    B,
    Start,
    Select,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub frame: u64,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepResult {
    pub observation: Observation,
    pub reward: f32,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub struct EnvConfig {
    pub action_frames: usize,
    pub render_pixels: bool,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            action_frames: 6,
            render_pixels: true,
        }
    }
}

pub struct GameboyEnv {
    rom: Vec<u8>,
    gameboy: Gameboy,
    config: EnvConfig,
}

impl GameboyEnv {
    pub fn new(rom: Vec<u8>, config: EnvConfig) -> Self {
        let mut gameboy = Gameboy::load(&rom);
        gameboy.ppu.headless = !config.render_pixels;

        Self {
            rom,
            gameboy,
            config,
        }
    }

    pub fn reset(&mut self) -> Observation {
        self.gameboy = Gameboy::load(&self.rom);
        self.gameboy.ppu.headless = !self.config.render_pixels;
        self.observation()
    }

    pub fn step(&mut self, action: Action) -> StepResult {
        self.gameboy.set_input(action.input());

        for _ in 0..self.config.action_frames {
            self.gameboy.run_frame();
        }

        self.gameboy.set_input(Input::empty());

        StepResult {
            observation: self.observation(),
            reward: 0.0,
            done: self.gameboy.stopped,
        }
    }

    pub fn gameboy(&self) -> &Gameboy {
        &self.gameboy
    }

    pub fn gameboy_mut(&mut self) -> &mut Gameboy {
        &mut self.gameboy
    }

    fn observation(&self) -> Observation {
        Observation {
            frame: self.gameboy.frames,
            pixels: if self.config.render_pixels {
                framebuffer_luma(&self.gameboy.ppu.framebuffer)
            } else {
                Vec::new()
            },
        }
    }
}

impl Action {
    fn input(self) -> Input {
        match self {
            Self::Noop => Input::empty(),
            Self::Left => Input::LEFT,
            Self::Right => Input::RIGHT,
            Self::Up => Input::UP,
            Self::Down => Input::DOWN,
            Self::A => Input::A,
            Self::B => Input::B,
            Self::Start => Input::START,
            Self::Select => Input::SELECT,
        }
    }
}

fn framebuffer_luma(framebuffer: &[u32]) -> Vec<u8> {
    framebuffer
        .iter()
        .map(|pixel| {
            let r = ((pixel >> 16) & 0xFF) as u16;
            let g = ((pixel >> 8) & 0xFF) as u16;
            let b = (pixel & 0xFF) as u16;
            ((r + g + b) / 3) as u8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_returns_initial_observation() {
        let mut env = GameboyEnv::new(test_rom(), EnvConfig::default());

        let observation = env.reset();

        assert_eq!(observation.frame, 0);
        assert_eq!(observation.pixels.len(), 160 * 144);
    }

    #[test]
    fn step_advances_configured_number_of_frames() {
        let mut env = GameboyEnv::new(
            test_rom(),
            EnvConfig {
                action_frames: 3,
                render_pixels: false,
            },
        );

        let step = env.step(Action::Noop);

        assert_eq!(step.observation.frame, 3);
        assert_eq!(env.gameboy().frames, 3);
        assert!(step.observation.pixels.is_empty());
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
