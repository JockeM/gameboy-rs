pub mod gameboy;
pub mod tetris;

pub use gameboy::{Action, EnvConfig, GameboyEnv, Observation, StepResult};
pub use tetris::{
    BoardMemoryMap, GameOverField, NumericEncoding, NumericField, TetrisAction, TetrisEnv,
    TetrisMemoryMap, TetrisObservation, TetrisState, TetrisStep, BOARD_HEIGHT, BOARD_WIDTH,
};
