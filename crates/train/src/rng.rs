#[derive(Clone, Debug)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.state
    }

    pub fn next_f32(&mut self) -> f32 {
        let value = self.next_u64() >> 40;
        value as f32 / (1_u32 << 24) as f32
    }

    pub fn next_usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            (self.next_u64() as usize) % upper
        }
    }
}

impl Default for SimpleRng {
    fn default() -> Self {
        Self::new(0xC0DE_5EED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic() {
        let mut left = SimpleRng::new(42);
        let mut right = SimpleRng::new(42);

        assert_eq!(left.next_u64(), right.next_u64());
        assert_eq!(left.next_u64(), right.next_u64());
    }

    #[test]
    fn next_usize_stays_in_range() {
        let mut rng = SimpleRng::new(42);

        for _ in 0..100 {
            assert!(rng.next_usize(7) < 7);
        }
    }
}
