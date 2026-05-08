use crate::gameboy::{Action, EnvConfig, GameboyEnv, Observation};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 17;

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
    pub occupied: [[bool; BOARD_WIDTH]; BOARD_HEIGHT],
    pub score: u32,
    pub lines: u32,
    pub level: u8,
    pub game_over: bool,
    pub pieces: Option<TetrisPieces>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TetrisBoardFeatures {
    pub column_heights: [usize; BOARD_WIDTH],
    pub aggregate_height: usize,
    pub max_height: usize,
    pub holes: usize,
    pub bumpiness: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TetrisMemoryMap {
    pub board: BoardMemoryMap,
    pub score: Option<NumericField>,
    pub lines: Option<NumericField>,
    pub level_addr: Option<u16>,
    pub game_over: Option<GameOverField>,
    pub pieces: Option<PieceMemoryMap>,
}

impl TetrisMemoryMap {
    pub const GAME_BOY_TETRIS: Self = Self {
        board: BoardMemoryMap::GAME_BOY_TETRIS,
        score: Some(NumericField {
            start: 0xC0A0,
            len: 3,
            encoding: NumericEncoding::Bcd,
        }),
        lines: None,
        level_addr: None,
        game_over: None,
        pieces: Some(PieceMemoryMap::GAME_BOY_TETRIS),
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoardMemoryMap {
    pub start: u16,
    pub row_stride: usize,
    pub empty_cell: u8,
}

impl BoardMemoryMap {
    pub const GAME_BOY_TETRIS: Self = Self {
        start: 0xC822,
        row_stride: 0x20,
        empty_cell: 0x2F,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PieceMemoryMap {
    pub current_piece: u16,
    pub preview_piece: u16,
    pub next_preview_piece: u16,
    pub center_x: u16,
    pub center_y: u16,
    pub lowest_right_x: u16,
    pub lowest_right_y: u16,
    pub moved_lowest_right_x: u16,
    pub moved_lowest_right_y: u16,
}

impl PieceMemoryMap {
    pub const GAME_BOY_TETRIS: Self = Self {
        current_piece: 0xC203,
        preview_piece: 0xC213,
        next_preview_piece: 0xFFAE,
        center_x: 0xC202,
        center_y: 0xC201,
        lowest_right_x: 0xFF92,
        lowest_right_y: 0xFF93,
        moved_lowest_right_x: 0xFFB3,
        moved_lowest_right_y: 0xFFB2,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TetrisPieces {
    pub current: TetrisPiece,
    pub preview: TetrisPiece,
    pub next_preview: TetrisPiece,
    pub center: Position,
    pub lowest_right: Position,
    pub moved_lowest_right: Position,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TetrisPiece {
    pub value: u8,
    pub kind: Tetromino,
    pub rotation: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tetromino {
    L,
    J,
    I,
    O,
    Z,
    S,
    T,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub x: u8,
    pub y: u8,
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
        let previous_state = self.current_state();
        let step = self.gameboy.step(action.gameboy_action());
        let observation = self.observation(step.observation);
        let done = step.done
            || observation
                .state
                .as_ref()
                .is_some_and(|state| state.game_over);
        let reward = match (previous_state.as_ref(), observation.state.as_ref()) {
            (Some(previous), Some(next)) => tetris_reward(previous, next),
            _ => step.reward,
        };

        TetrisStep {
            observation,
            reward,
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

    fn current_state(&self) -> Option<TetrisState> {
        self.memory_map.map(|memory_map| {
            TetrisState::from_memory(
                self.gameboy.gameboy().mem.as_slice(),
                self.gameboy.gameboy().frames,
                memory_map,
            )
        })
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
    pub fn from_memory(memory: &[u8], frame: u64, memory_map: TetrisMemoryMap) -> Self {
        let mut board = [[0; BOARD_WIDTH]; BOARD_HEIGHT];
        let mut occupied = [[false; BOARD_WIDTH]; BOARD_HEIGHT];

        for (y, row) in board.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                *cell = read_byte(memory, board_address(memory_map.board, y, x));
                occupied[y][x] = *cell != memory_map.board.empty_cell;
            }
        }

        Self {
            frame,
            board,
            occupied,
            score: memory_map
                .score
                .map_or(0, |field| read_numeric(memory, field)),
            lines: memory_map
                .lines
                .map_or(0, |field| read_numeric(memory, field)),
            level: memory_map
                .level_addr
                .map_or(0, |addr| read_byte(memory, usize::from(addr))),
            game_over: memory_map.game_over.is_some_and(|field| {
                read_byte(memory, usize::from(field.addr)) == field.active_value
            }),
            pieces: memory_map
                .pieces
                .map(|pieces| TetrisPieces::from_memory(memory, pieces)),
        }
    }

    pub fn board_features(&self) -> TetrisBoardFeatures {
        TetrisBoardFeatures::from_occupied(&self.occupied)
    }
}

impl TetrisBoardFeatures {
    pub fn from_occupied(occupied: &[[bool; BOARD_WIDTH]; BOARD_HEIGHT]) -> Self {
        let mut column_heights = [0; BOARD_WIDTH];
        let mut holes = 0;

        for x in 0..BOARD_WIDTH {
            let first_occupied = (0..BOARD_HEIGHT).find(|&y| occupied[y][x]);
            if let Some(first_occupied) = first_occupied {
                column_heights[x] = BOARD_HEIGHT - first_occupied;
                holes += ((first_occupied + 1)..BOARD_HEIGHT)
                    .filter(|&y| !occupied[y][x])
                    .count();
            }
        }

        let aggregate_height = column_heights.iter().sum();
        let max_height = column_heights.iter().copied().max().unwrap_or(0);
        let bumpiness = column_heights
            .windows(2)
            .map(|heights| heights[0].abs_diff(heights[1]))
            .sum();

        Self {
            column_heights,
            aggregate_height,
            max_height,
            holes,
            bumpiness,
        }
    }
}

fn tetris_reward(previous: &TetrisState, next: &TetrisState) -> f32 {
    if board_looks_uninitialized(previous) || board_looks_uninitialized(next) {
        return 0.0;
    }

    let previous_features = previous.board_features();
    let next_features = next.board_features();
    let score_delta = next.score.saturating_sub(previous.score) as f32;
    let lines_delta = next.lines.saturating_sub(previous.lines) as f32;

    let mut reward = score_delta * 0.01 + lines_delta * 10.0;
    reward += previous_features.holes as f32 - next_features.holes as f32;
    reward +=
        (previous_features.aggregate_height as f32 - next_features.aggregate_height as f32) * 0.1;
    reward += (previous_features.bumpiness as f32 - next_features.bumpiness as f32) * 0.2;

    if next.game_over && !previous.game_over {
        reward -= 100.0;
    }

    reward
}

fn board_looks_uninitialized(state: &TetrisState) -> bool {
    let first_cell = state.board[0][0];
    state.occupied.iter().flatten().all(|occupied| *occupied)
        && state.board.iter().flatten().all(|cell| *cell == first_cell)
}

impl TetrisPieces {
    fn from_memory(memory: &[u8], memory_map: PieceMemoryMap) -> Self {
        Self {
            current: TetrisPiece::from_value(read_byte(
                memory,
                usize::from(memory_map.current_piece),
            )),
            preview: TetrisPiece::from_value(read_byte(
                memory,
                usize::from(memory_map.preview_piece),
            )),
            next_preview: TetrisPiece::from_value(read_byte(
                memory,
                usize::from(memory_map.next_preview_piece),
            )),
            center: Position {
                x: read_byte(memory, usize::from(memory_map.center_x)),
                y: read_byte(memory, usize::from(memory_map.center_y)),
            },
            lowest_right: Position {
                x: read_byte(memory, usize::from(memory_map.lowest_right_x)),
                y: read_byte(memory, usize::from(memory_map.lowest_right_y)),
            },
            moved_lowest_right: Position {
                x: read_byte(memory, usize::from(memory_map.moved_lowest_right_x)),
                y: read_byte(memory, usize::from(memory_map.moved_lowest_right_y)),
            },
        }
    }
}

impl TetrisPiece {
    fn from_value(value: u8) -> Self {
        Self {
            value,
            kind: tetromino_from_value(value),
            rotation: value & 0x03,
        }
    }
}

fn tetromino_from_value(value: u8) -> Tetromino {
    match value / 4 {
        0 => Tetromino::L,
        1 => Tetromino::J,
        2 => Tetromino::I,
        3 => Tetromino::O,
        4 => Tetromino::Z,
        5 => Tetromino::S,
        6 => Tetromino::T,
        _ => Tetromino::Unknown,
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

fn board_address(board: BoardMemoryMap, row: usize, col: usize) -> usize {
    usize::from(board.start) + row * board.row_stride + col
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
            board: BoardMemoryMap {
                start: 0xC000,
                row_stride: 0x20,
                empty_cell: 0x2F,
            },
            score: Some(NumericField {
                start: 0xC100,
                len: 3,
                encoding: NumericEncoding::Bcd,
            }),
            lines: Some(NumericField {
                start: 0xC103,
                len: 2,
                encoding: NumericEncoding::LittleEndian,
            }),
            level_addr: Some(0xC105),
            game_over: Some(GameOverField {
                addr: 0xC106,
                active_value: 0xFF,
            }),
            pieces: None,
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
            mem[0xC000] = 0x2F;
            mem[0xC001] = 0x2F;
            mem[0xC000] = 7;
            mem[0xC000 + 0x20 + 1] = 8;
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
        assert!(state.occupied[0][0]);
        assert!(state.occupied[1][1]);
        assert!(!state.occupied[0][1]);
        assert_eq!(state.score, 123456);
        assert_eq!(state.lines, 42);
        assert_eq!(state.level, 9);
        assert!(state.game_over);
        assert!(step.done);
    }

    #[test]
    fn game_boy_tetris_board_map_matches_known_row_addresses() {
        let board = BoardMemoryMap::GAME_BOY_TETRIS;

        assert_eq!(board_address(board, 0, 0), 0xC822);
        assert_eq!(board_address(board, 0, 9), 0xC82B);
        assert_eq!(board_address(board, 1, 0), 0xC842);
        assert_eq!(board_address(board, 16, 9), 0xCA2B);
    }

    #[test]
    fn extracts_game_boy_tetris_piece_state() {
        let mut memory = [0; 0x10000];
        memory[0xC203] = 0x1A;
        memory[0xC213] = 0x08;
        memory[0xFFAE] = 0x0C;
        memory[0xC202] = 5;
        memory[0xC201] = 6;
        memory[0xFF92] = 7;
        memory[0xFF93] = 8;
        memory[0xFFB3] = 9;
        memory[0xFFB2] = 10;

        let pieces = TetrisPieces::from_memory(&memory, PieceMemoryMap::GAME_BOY_TETRIS);

        assert_eq!(
            pieces.current,
            TetrisPiece {
                value: 0x1A,
                kind: Tetromino::T,
                rotation: 2,
            }
        );
        assert_eq!(pieces.preview.kind, Tetromino::I);
        assert_eq!(pieces.next_preview.kind, Tetromino::O);
        assert_eq!(pieces.center, Position { x: 5, y: 6 });
        assert_eq!(pieces.lowest_right, Position { x: 7, y: 8 });
        assert_eq!(pieces.moved_lowest_right, Position { x: 9, y: 10 });
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

    #[test]
    fn board_features_count_heights_holes_and_bumpiness() {
        let mut occupied = [[false; BOARD_WIDTH]; BOARD_HEIGHT];
        occupied[BOARD_HEIGHT - 3][0] = true;
        occupied[BOARD_HEIGHT - 1][0] = true;
        occupied[BOARD_HEIGHT - 1][1] = true;

        let features = TetrisBoardFeatures::from_occupied(&occupied);

        assert_eq!(features.column_heights[0], 3);
        assert_eq!(features.column_heights[1], 1);
        assert_eq!(features.aggregate_height, 4);
        assert_eq!(features.max_height, 3);
        assert_eq!(features.holes, 1);
        assert_eq!(features.bumpiness, 3);
    }

    #[test]
    fn tetris_reward_prefers_score_and_cleaner_board() {
        let mut previous = empty_state();
        previous.score = 100;
        previous.lines = 1;
        previous.occupied[BOARD_HEIGHT - 3][0] = true;
        previous.occupied[BOARD_HEIGHT - 1][0] = true;

        let mut next = empty_state();
        next.score = 200;
        next.lines = 2;
        next.occupied[BOARD_HEIGHT - 1][0] = true;

        assert!(tetris_reward(&previous, &next) > 0.0);
    }

    #[test]
    fn tetris_reward_penalizes_new_game_over() {
        let previous = empty_state();
        let mut next = empty_state();
        next.game_over = true;

        assert_eq!(tetris_reward(&previous, &next), -100.0);
    }

    #[test]
    fn tetris_reward_ignores_uninitialized_playfield_memory() {
        let mut previous = empty_state();
        previous.board = [[0; BOARD_WIDTH]; BOARD_HEIGHT];
        previous.occupied = [[true; BOARD_WIDTH]; BOARD_HEIGHT];

        let next = empty_state();

        assert_eq!(tetris_reward(&previous, &next), 0.0);
    }

    fn test_rom() -> Vec<u8> {
        let mut rom = vec![0; 0x150];
        rom[0x100] = 0x00;
        rom[0x101] = 0x18;
        rom[0x102] = 0xFD;
        rom
    }

    fn empty_state() -> TetrisState {
        TetrisState {
            frame: 0,
            board: [[0; BOARD_WIDTH]; BOARD_HEIGHT],
            occupied: [[false; BOARD_WIDTH]; BOARD_HEIGHT],
            score: 0,
            lines: 0,
            level: 0,
            game_over: false,
            pieces: None,
        }
    }
}
