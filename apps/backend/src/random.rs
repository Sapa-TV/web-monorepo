use rand::RngExt;

use crate::roulette::machine::RandomProvider;

#[derive(Clone)]
#[non_exhaustive]
pub struct StandartRandomProvider;

impl StandartRandomProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StandartRandomProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomProvider for StandartRandomProvider {
    fn next(&self) -> f64 {
        rand::rng().random()
    }
}
