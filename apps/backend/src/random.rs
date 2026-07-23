use rand::RngExt;

use crate::roulette::machine::RandomProvider;

pub struct StandartRandomProvider;

impl RandomProvider for StandartRandomProvider {
    fn next(&self) -> f64 {
        rand::rng().random()
    }
}
