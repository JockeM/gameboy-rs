use env::{TetrisAction, TetrisObservation};

pub trait Agent {
    fn act(&mut self, observation: &TetrisObservation) -> TetrisAction;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopAgent;

impl Agent for NoopAgent {
    fn act(&mut self, _observation: &TetrisObservation) -> TetrisAction {
        TetrisAction::Noop
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StartThenNoopAgent {
    remaining_start_steps: usize,
}

impl StartThenNoopAgent {
    pub fn new(start_steps: usize) -> Self {
        Self {
            remaining_start_steps: start_steps,
        }
    }
}

impl Agent for StartThenNoopAgent {
    fn act(&mut self, _observation: &TetrisObservation) -> TetrisAction {
        if self.remaining_start_steps == 0 {
            TetrisAction::Noop
        } else {
            self.remaining_start_steps -= 1;
            TetrisAction::Start
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env::Observation;

    #[test]
    fn noop_agent_returns_noop_action() {
        let mut agent = NoopAgent;
        let observation = TetrisObservation {
            gameboy: Observation {
                frame: 0,
                pixels: Vec::new(),
            },
            state: None,
        };

        assert_eq!(agent.act(&observation), TetrisAction::Noop);
    }

    #[test]
    fn start_then_noop_agent_presses_start_for_configured_steps() {
        let mut agent = StartThenNoopAgent::new(2);
        let observation = TetrisObservation {
            gameboy: Observation {
                frame: 0,
                pixels: Vec::new(),
            },
            state: None,
        };

        assert_eq!(agent.act(&observation), TetrisAction::Start);
        assert_eq!(agent.act(&observation), TetrisAction::Start);
        assert_eq!(agent.act(&observation), TetrisAction::Noop);
    }
}
