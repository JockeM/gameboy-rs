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
}
