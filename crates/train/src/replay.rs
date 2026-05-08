use crate::model::STATE_FEATURES;
use crate::rng::SimpleRng;

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub state: [f32; STATE_FEATURES],
    pub action: usize,
    pub reward: f32,
    pub next_state: [f32; STATE_FEATURES],
    pub done: bool,
}

#[derive(Clone, Debug)]
pub struct ReplayBuffer {
    transitions: Vec<Transition>,
    capacity: usize,
    next_index: usize,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        assert!(
            capacity > 0,
            "replay buffer capacity must be greater than 0"
        );
        Self {
            transitions: Vec::with_capacity(capacity),
            capacity,
            next_index: 0,
        }
    }

    pub fn push(&mut self, transition: Transition) {
        if self.transitions.len() < self.capacity {
            self.transitions.push(transition);
        } else {
            self.transitions[self.next_index] = transition;
        }

        self.next_index = (self.next_index + 1) % self.capacity;
    }

    pub fn len(&self) -> usize {
        self.transitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn sample(&self, batch_size: usize, rng: &mut SimpleRng) -> Vec<Transition> {
        let size = batch_size.min(self.transitions.len());
        let mut batch = Vec::with_capacity(size);

        for _ in 0..size {
            let index = rng.next_usize(self.transitions.len());
            batch.push(self.transitions[index].clone());
        }

        batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_buffer_keeps_capacity() {
        let mut buffer = ReplayBuffer::new(2);

        buffer.push(transition(0.0));
        buffer.push(transition(1.0));
        buffer.push(transition(2.0));

        assert_eq!(buffer.len(), 2);
        assert!(buffer.sample(8, &mut SimpleRng::new(1)).len() <= 2);
    }

    #[test]
    fn replay_buffer_samples_requested_size_when_available() {
        let mut buffer = ReplayBuffer::new(4);
        let mut rng = SimpleRng::new(1);

        buffer.push(transition(0.0));
        buffer.push(transition(1.0));
        buffer.push(transition(2.0));

        assert_eq!(buffer.sample(2, &mut rng).len(), 2);
    }

    fn transition(value: f32) -> Transition {
        let mut state = [0.0; STATE_FEATURES];
        state[0] = value;

        Transition {
            state,
            action: 0,
            reward: value,
            next_state: state,
            done: false,
        }
    }
}
