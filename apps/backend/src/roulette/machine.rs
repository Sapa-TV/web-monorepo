use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotService};

pub trait RandomProvider {
    fn next(&self) -> f64;
}

#[non_exhaustive]
pub struct RouletteService<Rand: RandomProvider, Repo: RouletteSlotRepository> {
    slot_service: RouletteSlotService<Repo>,
    random: Rand,
}

impl<Rand: RandomProvider, Repo: RouletteSlotRepository> RouletteService<Rand, Repo> {
    pub fn new(slot_service: RouletteSlotService<Repo>, random: Rand) -> Self {
        Self {
            slot_service,
            random,
        }
    }

    pub fn roll(&self) -> Option<&RouletteSlot> {
        let total_weight: u64 = self.slot_service.total_weight();
        let random_value = self.random.next() * total_weight as f64;

        self.slot_service.get_slot_by_weight(random_value as u64)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
    use crate::roulette::rarity::RarityId;
    use crate::roulette::slot_service::RouletteSlotId;

    const COMMON: RarityId = RarityId::new(1);

    use super::*;

    struct MockRandomProvider {
        value: Cell<f64>,
    }

    impl MockRandomProvider {
        fn new(value: f64) -> Self {
            Self {
                value: Cell::new(value),
            }
        }
    }

    impl RandomProvider for MockRandomProvider {
        fn next(&self) -> f64 {
            self.value.get()
        }
    }

    #[tokio::test]
    async fn test_absolute_favorite_case() {
        let mock_random = MockRandomProvider::new(0.99);
        let slots = vec![
            RouletteSlot::new(
                RouletteSlotId::new(0),
                "Test_Loser_1",
                COMMON,
                0,
                "Loser 1 Action",
            ),
            RouletteSlot::new(
                RouletteSlotId::new(0),
                "Test_Winner",
                COMMON,
                100,
                "Winner Action",
            ),
            RouletteSlot::new(
                RouletteSlotId::new(0),
                "Test_Loser_2",
                COMMON,
                0,
                "Loser 1 Action",
            ),
        ];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let slot_service = RouletteSlotService::build(repo).await.unwrap();
        let roulette = RouletteService::new(slot_service, mock_random);

        let winner_slot = roulette.roll().unwrap();
        assert_eq!(winner_slot.name, "Test_Winner".to_string());
    }

    #[tokio::test]
    async fn test_mid_boundary_switching() {
        let mock_random = MockRandomProvider::new(0.0);
        let slots = vec![
            RouletteSlot::new(
                RouletteSlotId::new(0),
                "Test_Variant_A",
                COMMON,
                10,
                "Variant A Action",
            ),
            RouletteSlot::new(
                RouletteSlotId::new(0),
                "Test_Variant_B",
                COMMON,
                20,
                "Variant B Action",
            ),
        ];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let slot_service = RouletteSlotService::build(repo).await.unwrap();
        let roulette = RouletteService::new(slot_service, mock_random);

        let winner_slot = roulette.roll().unwrap();
        assert_eq!(winner_slot.name, "Test_Variant_A".to_string());

        roulette.random.value.set(0.33);
        let winner_slot = roulette.roll().unwrap();
        assert_eq!(winner_slot.name, "Test_Variant_A".to_string());

        roulette.random.value.set(0.34);
        let winner_slot = roulette.roll().unwrap();
        assert_eq!(winner_slot.name, "Test_Variant_B".to_string());
    }
}
