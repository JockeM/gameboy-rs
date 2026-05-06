use crate::gameboy::{Action, EnvConfig, GameboyEnv, Observation};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

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
    pub state: Option<TetrisState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TetrisStep {
    pub observation: TetrisObservation,
    pub reward: f32,
    pub done: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TetrisState {
    pub frame: u64,
    pub board: [[u8; BOARD_WIDTH]; BOARD_HEIGHT],
    pub score: u32,
    pub lines: u32,
    pub level: u8,
    pub game_over: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TetrisMemoryMap {
    pub board_start: u16,
    pub score: NumericField,
    pub lines: NumericField,
    pub level_addr: u16,
    pub game_over: GameOverField,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumericField {
    pub start: u16,
    pub len: usize,
    pub encoding: NumericEncoding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericEncoding {
    LittleEndian,
    BigEndian,
    Bcd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameOverField {
    pub addr: u16,
    pub active_value: u8,
}

pub struct TetrisEnv {
    gameboy: GameboyEnv,
    memory_map: Option<TetrisMemoryMap>,
}

impl TetrisEnv {
    pub fn new(rom: Vec<u8>, config: EnvConfig) -> Self {
        Self {
            gameboy: GameboyEnv::new(rom, config),
            memory_map: None,
        }
    }

    pub fn new_with_memory_map(
        rom: Vec<u8>,
        config: EnvConfig,
        memory_map: TetrisMemoryMap,
    ) -> Self {
        Self {
            gameboy: GameboyEnv::new(rom, config),
            memory_map: Some(memory_map),
        }
    }

    pub fn reset(&mut self) -> TetrisObservation {
        let gameboy = self.gameboy.reset();
        self.observation(gameboy)
    }

    pub fn step(&mut self, action: TetrisAction) -> TetrisStep {
        let step = self.gameboy.step(action.gameboy_action());
        let observation = self.observation(step.observation);
        let done = step.done
            || observation
                .state
                .as_ref()
                .is_some_and(|state| state.game_over);

        TetrisStep {
            observation,
            reward: step.reward,
            done,
        }
    }

    pub fn gameboy(&self) -> &GameboyEnv {
        &self.gameboy
    }

    pub fn gameboy_mut(&mut self) -> &mut GameboyEnv {
        &mut self.gameboy
    }

    fn observation(&self, gameboy: Observation) -> TetrisObservation {
        TetrisObservation {
            state: self.memory_map.map(|memory_map| {
                TetrisState::from_memory(
                    self.gameboy.gameboy().mem.as_slice(),
                    gameboy.frame,
                    memory_map,
                )
            }),
            gameboy,
        }
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

impl TetrisState {
    fn from_memory(memory: &[u8], frame: u64, memory_map: TetrisMemoryMap) -> Self {
        let mut board = [[0; BOARD_WIDTH]; BOARD_HEIGHT];

        for (y, row) in board.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let offset = y * BOARD_WIDTH + x;
                *cell = read_byte(memory, usize::from(memory_map.board_start) + offset);
            }
        }

        Self {
            frame,
            board,
            score: read_numeric(memory, memory_map.score),
            lines: read_numeric(memory, memory_map.lines),
            level: read_byte(memory, usize::from(memory_map.level_addr)),
            game_over: read_byte(memory, usize::from(memory_map.game_over.addr))
                == memory_map.game_over.active_value,
        }
    }
}

fn read_numeric(memory: &[u8], field: NumericField) -> u32 {
    match field.encoding {
        NumericEncoding::LittleEndian => (0..field.len).fold(0, |value, offset| {
            value
                | (u32::from(read_byte(memory, usize::from(field.start) + offset)) << (offset * 8))
        }),
        NumericEncoding::BigEndian => (0..field.len).fold(0, |value, offset| {
            (value << 8) | u32::from(read_byte(memory, usize::from(field.start) + offset))
        }),
        NumericEncoding::Bcd => (0..field.len).fold(0, |value, offset| {
            let byte = read_byte(memory, usize::from(field.start) + offset);
            value * 100 + u32::from(byte >> 4) * 10 + u32::from(byte & 0x0F)
        }),
    }
}

fn read_byte(memory: &[u8], address: usize) -> u8 {
    memory.get(address).copied().unwrap_or(0)
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
        assert_eq!(step.observation.state, None);
        assert_eq!(step.reward, 0.0);
        assert!(!step.done);
    }

    #[test]
    fn extracts_tetris_state_from_configured_memory_map() {
        let memory_map = TetrisMemoryMap {
            board_start: 0xC000,
            score: NumericField {
                start: 0xC100,
                len: 3,
                encoding: NumericEncoding::Bcd,
            },
            lines: NumericField {
                start: 0xC103,
                len: 2,
                encoding: NumericEncoding::LittleEndian,
            },
            level_addr: 0xC105,
            game_over: GameOverField {
                addr: 0xC106,
                active_value: 0xFF,
            },
        };
        let mut env = TetrisEnv::new_with_memory_map(
            test_rom(),
            EnvConfig {
                action_frames: 1,
                render_pixels: false,
            },
            memory_map,
        );

        {
            let mem = &mut env.gameboy_mut().gameboy_mut().mem;
            mem[0xC000] = 7;
            mem[0xC000 + BOARD_WIDTH + 1] = 8;
            mem[0xC100] = 0x12;
            mem[0xC101] = 0x34;
            mem[0xC102] = 0x56;
            mem[0xC103] = 0x2A;
            mem[0xC104] = 0x00;
            mem[0xC105] = 9;
            mem[0xC106] = 0xFF;
        }

        let step = env.step(TetrisAction::Noop);
        let state = step.observation.state.expect("tetris state");

        assert_eq!(state.frame, 1);
        assert_eq!(state.board[0][0], 7);
        assert_eq!(state.board[1][1], 8);
        assert_eq!(state.score, 123456);
        assert_eq!(state.lines, 42);
        assert_eq!(state.level, 9);
        assert!(state.game_over);
        assert!(step.done);
    }

    #[test]
    fn reads_big_endian_numeric_fields() {
        let memory = [0x12, 0x34, 0x56];
        let value = read_numeric(
            &memory,
            NumericField {
                start: 0,
                len: 3,
                encoding: NumericEncoding::BigEndian,
            },
        );

        assert_eq!(value, 0x123456);
    }

    fn test_rom() -> Vec<u8> {
        let mut rom = vec![0; 0x150];
        rom[0x100] = 0x00;
        rom[0x101] = 0x18;
        rom[0x102] = 0xFD;
        rom
    }
}
