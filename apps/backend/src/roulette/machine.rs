use crate::roulette::slot_service::{RouletteSlot, RouletteSlotService};

pub trait RandomProvider {
    fn next(&self) -> f64;
}

pub struct RouletteService<R: RandomProvider> {
    slot_service: RouletteSlotService,
    random: R,
}

impl<R: RandomProvider> RouletteService<R> {
    pub fn new(slot_service: RouletteSlotService, random: R) -> Self {
        Self {
            slot_service,
            random,
        }
    }

    pub fn roll(&self) -> Option<&RouletteSlot> {
        let total_weight: u64 = self.slot_service.total_weight();
        let random_value = self.random.next() * total_weight as f64;

        let slot = self.slot_service.get_slot_by_weight(random_value as u64);

        slot
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use crate::roulette::slot_service::RouletteSlotRarity;

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

    #[test]
    fn test_absolute_favorite_case() {
        let mock_random = MockRandomProvider::new(0.99);
        let slots = vec![
            RouletteSlot::new(
                "Test_Loser_1",
                RouletteSlotRarity::Common,
                0,
                "Loser 1 Action",
            ),
            RouletteSlot::new(
                "Test_Winner",
                RouletteSlotRarity::Common,
                100,
                "Winner Action",
            ),
            RouletteSlot::new(
                "Test_Loser_2",
                RouletteSlotRarity::Common,
                0,
                "Loser 1 Action",
            ),
        ];
        let slot_service = RouletteSlotService::new(slots);
        let roulette = RouletteService::new(slot_service, mock_random);

        let winner_slot = roulette.roll().unwrap();
        assert_eq!(winner_slot.name, "Test_Winner".to_string());
    }

    #[test]
    fn test_mid_boundary_switching() {
        let mock_random = MockRandomProvider::new(0.0);
        let slots = vec![
            RouletteSlot::new(
                "Test_Variant_A",
                RouletteSlotRarity::Common,
                10,
                "Variant A Action",
            ),
            RouletteSlot::new(
                "Test_Variant_B",
                RouletteSlotRarity::Common,
                20,
                "Variant B Action",
            ),
        ];
        let slot_service = RouletteSlotService::new(slots);
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
