use burn::module::Module;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::Backend;
use burn::tensor::{Tensor, activation::relu};
use env::{BOARD_HEIGHT, BOARD_WIDTH, TetrisAction, TetrisObservation};

pub const STATE_FEATURES: usize = BOARD_WIDTH * BOARD_HEIGHT + 5;
pub const ACTIONS: usize = 7;

#[derive(Module, Debug)]
pub struct TetrisQNetwork<B: Backend> {
    input: Linear<B>,
    hidden: Linear<B>,
    output: Linear<B>,
}

impl<B: Backend> TetrisQNetwork<B> {
    pub fn new(device: &B::Device) -> Self {
        Self {
            input: LinearConfig::new(STATE_FEATURES, 128).init(device),
            hidden: LinearConfig::new(128, 64).init(device),
            output: LinearConfig::new(64, ACTIONS).init(device),
        }
    }

    pub fn forward(&self, state: Tensor<B, 2>) -> Tensor<B, 2> {
        let state = relu(self.input.forward(state));
        let state = relu(self.hidden.forward(state));
        self.output.forward(state)
    }
}

pub fn encode_observation(observation: &TetrisObservation) -> [f32; STATE_FEATURES] {
    let mut encoded = [0.0; STATE_FEATURES];

    let Some(state) = observation.state.as_ref() else {
        return encoded;
    };

    for y in 0..BOARD_HEIGHT {
        for x in 0..BOARD_WIDTH {
            encoded[y * BOARD_WIDTH + x] = if state.occupied[y][x] { 1.0 } else { 0.0 };
        }
    }

    let features = state.board_features();
    let offset = BOARD_WIDTH * BOARD_HEIGHT;
    encoded[offset] = features.aggregate_height as f32 / (BOARD_WIDTH * BOARD_HEIGHT) as f32;
    encoded[offset + 1] = features.max_height as f32 / BOARD_HEIGHT as f32;
    encoded[offset + 2] = features.holes as f32 / (BOARD_WIDTH * BOARD_HEIGHT) as f32;
    encoded[offset + 3] = features.bumpiness as f32 / (BOARD_WIDTH * BOARD_HEIGHT) as f32;
    encoded[offset + 4] = state.score as f32 / 100_000.0;

    encoded
}

pub fn action_from_index(index: usize) -> Option<TetrisAction> {
    match index {
        0 => Some(TetrisAction::Noop),
        1 => Some(TetrisAction::Left),
        2 => Some(TetrisAction::Right),
        3 => Some(TetrisAction::Down),
        4 => Some(TetrisAction::RotateClockwise),
        5 => Some(TetrisAction::RotateCounterClockwise),
        6 => Some(TetrisAction::Start),
        _ => None,
    }
}

pub fn action_index(action: TetrisAction) -> usize {
    match action {
        TetrisAction::Noop => 0,
        TetrisAction::Left => 1,
        TetrisAction::Right => 2,
        TetrisAction::Down => 3,
        TetrisAction::RotateClockwise => 4,
        TetrisAction::RotateCounterClockwise => 5,
        TetrisAction::Start => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use burn::tensor::Shape;
    use env::{Observation, TetrisState};

    type TestBackend = NdArray<f32>;

    #[test]
    fn q_network_forward_returns_one_value_per_action() {
        let device = Default::default();
        let model = TetrisQNetwork::<TestBackend>::new(&device);
        let input = Tensor::<TestBackend, 2>::zeros(Shape::new([1, STATE_FEATURES]), &device);

        let output = model.forward(input);

        assert_eq!(output.shape().dims(), [1, ACTIONS]);
    }

    #[test]
    fn encodes_board_and_summary_features() {
        let mut state = empty_state();
        state.occupied[BOARD_HEIGHT - 1][0] = true;
        state.score = 500;
        let observation = TetrisObservation {
            gameboy: Observation {
                frame: 0,
                pixels: Vec::new(),
            },
            state: Some(state),
        };

        let encoded = encode_observation(&observation);

        assert_eq!(encoded[(BOARD_HEIGHT - 1) * BOARD_WIDTH], 1.0);
        assert!(encoded[BOARD_WIDTH * BOARD_HEIGHT] > 0.0);
        assert_eq!(encoded[STATE_FEATURES - 1], 0.005);
    }

    #[test]
    fn action_indices_round_trip() {
        for index in 0..ACTIONS {
            let action = action_from_index(index).expect("action");
            assert_eq!(action_index(action), index);
        }

        assert_eq!(action_from_index(ACTIONS), None);
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
