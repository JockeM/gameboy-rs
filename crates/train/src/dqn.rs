use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use burn::prelude::Backend;
use burn::tensor::backend::AutodiffBackend;
use burn::tensor::{Int, Tensor, TensorData};

use crate::model::{ACTIONS, STATE_FEATURES, TetrisQNetwork};
use crate::replay::Transition;

pub const DEFAULT_LEARNING_RATE: f64 = 1e-4;
pub const DEFAULT_DISCOUNT: f32 = 0.99;

pub struct DqnTrainer<B: AutodiffBackend> {
    optimizer: burn::optim::adaptor::OptimizerAdaptor<burn::optim::Adam, TetrisQNetwork<B>, B>,
    learning_rate: f64,
    discount: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrainStepStats {
    pub loss: f32,
}

impl<B: AutodiffBackend> DqnTrainer<B> {
    pub fn new(learning_rate: f64, discount: f32) -> Self {
        Self {
            optimizer: AdamConfig::new().init(),
            learning_rate,
            discount,
        }
    }

    pub fn train_step(
        &mut self,
        model: TetrisQNetwork<B>,
        batch: &[Transition],
        device: &B::Device,
    ) -> (TetrisQNetwork<B>, TrainStepStats) {
        assert!(!batch.is_empty(), "DQN batch must not be empty");

        let batch_size = batch.len();
        let states = state_tensor::<B>(batch.iter().map(|transition| transition.state), device);
        let next_states =
            state_tensor::<B>(batch.iter().map(|transition| transition.next_state), device);
        let actions = Tensor::<B, 1, Int>::from_data(
            TensorData::new(
                batch
                    .iter()
                    .map(|transition| transition.action as i32)
                    .collect::<Vec<_>>(),
                [batch_size],
            ),
            device,
        );

        let q_values = model.forward(states);
        let action_mask: Tensor<B, 2> = actions.one_hot::<2>(ACTIONS).float();
        let selected_q_values = (q_values * action_mask).sum_dim(1);

        let next_q_values = model.forward(next_states).detach();
        let next_max_values = next_q_values
            .max_dim(1)
            .into_data()
            .to_vec::<f32>()
            .expect("next q-values should be f32");
        let targets = batch
            .iter()
            .zip(next_max_values)
            .map(|(transition, next_max)| {
                if transition.done {
                    transition.reward
                } else {
                    transition.reward + self.discount * next_max
                }
            })
            .collect::<Vec<_>>();
        let targets = Tensor::<B, 2>::from_data(TensorData::new(targets, [batch_size, 1]), device);

        let loss = (selected_q_values - targets).powi_scalar(2).mean();
        let loss_value = loss
            .clone()
            .into_data()
            .to_vec::<f32>()
            .expect("loss should be f32")[0];
        let grads = GradientsParams::from_grads(loss.backward(), &model);
        let model = self.optimizer.step(self.learning_rate, model, grads);

        (model, TrainStepStats { loss: loss_value })
    }
}

fn state_tensor<B: Backend>(
    states: impl Iterator<Item = [f32; STATE_FEATURES]>,
    device: &B::Device,
) -> Tensor<B, 2> {
    let states = states.collect::<Vec<_>>();
    let batch_size = states.len();
    let data = states.into_iter().flatten().collect::<Vec<_>>();

    Tensor::<B, 2>::from_data(TensorData::new(data, [batch_size, STATE_FEATURES]), device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::{Autodiff, NdArray};

    type TestBackend = Autodiff<NdArray<f32>>;

    #[test]
    fn dqn_trainer_updates_model_and_reports_loss() {
        let device = Default::default();
        let model = TetrisQNetwork::<TestBackend>::new(&device);
        let mut trainer = DqnTrainer::<TestBackend>::new(DEFAULT_LEARNING_RATE, DEFAULT_DISCOUNT);

        let (model, stats) = trainer.train_step(model, &batch(), &device);

        assert!(stats.loss.is_finite());
        let _ = model;
    }

    fn batch() -> Vec<Transition> {
        let mut state = [0.0; STATE_FEATURES];
        state[0] = 1.0;
        let mut next_state = [0.0; STATE_FEATURES];
        next_state[1] = 1.0;

        vec![
            Transition {
                state,
                action: 0,
                reward: 1.0,
                next_state,
                done: false,
            },
            Transition {
                state: next_state,
                action: 1,
                reward: -1.0,
                next_state: state,
                done: true,
            },
        ]
    }
}
