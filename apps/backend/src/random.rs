use rand::RngExt;

use crate::roulette::machine::RandomProvider;

#[derive(Clone)]
#[non_exhaustive]
pub struct StandartRandomProvider;

impl RandomProvider for StandartRandomProvider {
    fn next(&self) -> f64 {
        rand::rng().random()
    }
}
