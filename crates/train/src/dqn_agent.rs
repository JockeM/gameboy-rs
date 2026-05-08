use agent::Agent;
use burn::prelude::Backend;
use burn::tensor::{Tensor, TensorData};
use env::{TetrisAction, TetrisObservation};

use crate::model::{
    ACTIONS, STATE_FEATURES, TetrisQNetwork, action_from_index, encode_observation,
};
use crate::rng::SimpleRng;

#[derive(Debug)]
pub struct BurnDqnAgent<B: Backend> {
    model: TetrisQNetwork<B>,
    device: B::Device,
    epsilon: f32,
    rng: SimpleRng,
    remaining_start_steps: usize,
}

impl<B: Backend> BurnDqnAgent<B> {
    pub fn new(
        model: TetrisQNetwork<B>,
        device: B::Device,
        epsilon: f32,
        seed: u64,
        start_steps: usize,
    ) -> Self {
        Self {
            model,
            device,
            epsilon,
            rng: SimpleRng::new(seed),
            remaining_start_steps: start_steps,
        }
    }

    pub fn model(&self) -> &TetrisQNetwork<B> {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut TetrisQNetwork<B> {
        &mut self.model
    }

    pub fn set_model(&mut self, model: TetrisQNetwork<B>) {
        self.model = model;
    }

    pub fn set_epsilon(&mut self, epsilon: f32) {
        self.epsilon = epsilon.clamp(0.0, 1.0);
    }

    fn greedy_action(&self, observation: &TetrisObservation) -> TetrisAction {
        let encoded = encode_observation(observation);
        let tensor = Tensor::<B, 2>::from_data(
            TensorData::new(encoded.to_vec(), [1, STATE_FEATURES]),
            &self.device,
        );
        let values = self
            .model
            .forward(tensor)
            .into_data()
            .to_vec::<f32>()
            .expect("q-values should be f32");

        let best = values
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
            .unwrap_or(0);

        action_from_index(best).unwrap_or(TetrisAction::Noop)
    }

    fn random_action(&mut self) -> TetrisAction {
        action_from_index(self.rng.next_usize(ACTIONS)).unwrap_or(TetrisAction::Noop)
    }
}

impl<B: Backend> Agent for BurnDqnAgent<B> {
    fn act(&mut self, observation: &TetrisObservation) -> TetrisAction {
        if self.remaining_start_steps > 0 {
            self.remaining_start_steps -= 1;
            return TetrisAction::Start;
        }

        if self.rng.next_f32() < self.epsilon {
            self.random_action()
        } else {
            self.greedy_action(observation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use env::Observation;

    type TestBackend = NdArray<f32>;

    #[test]
    fn dqn_agent_presses_start_before_model_actions() {
        let device = Default::default();
        let model = TetrisQNetwork::<TestBackend>::new(&device);
        let mut agent = BurnDqnAgent::new(model, device, 0.0, 1, 2);
        let observation = TetrisObservation {
            gameboy: Observation {
                frame: 0,
                pixels: Vec::new(),
            },
            state: None,
        };

        assert_eq!(agent.act(&observation), TetrisAction::Start);
        assert_eq!(agent.act(&observation), TetrisAction::Start);
        assert!(matches!(
            agent.act(&observation),
            TetrisAction::Noop
                | TetrisAction::Left
                | TetrisAction::Right
                | TetrisAction::Down
                | TetrisAction::RotateClockwise
                | TetrisAction::RotateCounterClockwise
                | TetrisAction::Start
        ));
    }
}
