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

pub fn generate_secret() -> String {
    let hi: u128 = rand::rng().random();
    let lo: u128 = rand::rng().random();
    format!("{hi:032x}{lo:032x}")
}
