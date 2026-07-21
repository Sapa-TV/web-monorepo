#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum RouletteSlotRarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
    Mythical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouletteSlotId(u64);

#[derive(Debug, Clone)]
pub struct RouletteSlot {
    pub(super) id: RouletteSlotId,
    pub(super) name: String,
    pub(super) rarity: RouletteSlotRarity,
    pub(super) weight: u64,
    pub(super) action: String,
}

impl RouletteSlot {
    pub fn new(name: &str, rarity: RouletteSlotRarity, weight: u64, action: &str) -> Self {
        Self {
            id: RouletteSlotId(0),
            name: name.to_string(),
            rarity,
            weight,
            action: action.to_string(),
        }
    }
}

pub struct RouletteSlotService {
    slots: Vec<RouletteSlot>,
}

impl RouletteSlotService {
    pub fn new(slots: Vec<RouletteSlot>) -> Self {
        Self { slots }
    }

    pub fn add_slot(&mut self, slot: RouletteSlot) {
        self.slots.push(slot);
    }

    pub fn get_slots(&self) -> &Vec<RouletteSlot> {
        &self.slots
    }

    pub fn edit_slot(&mut self, _slot: RouletteSlot) {
        // TODO: Implement edit slot
    }

    pub fn total_weight(&self) -> u64 {
        self.slots.iter().map(|slot| slot.weight).sum()
    }

    pub fn get_slot_by_weight(&self, weight: u64) -> Option<&RouletteSlot> {
        let mut current_weight = 0;
        for slot in &self.slots {
            current_weight += slot.weight;
            if weight < current_weight {
                return Some(slot);
            }
        }

        let slot = self.slots.iter().max_by_key(|slot| slot.weight);
        slot
    }
}

#[cfg(test)]
mod tests {
    use crate::roulette::slot_service::RouletteSlotRarity::Common;

    use super::*;

    #[test]
    fn test_total_weight_calc() {
        let slots = vec![
            RouletteSlot::new("Test_1", Common, 123, "Action 1"),
            RouletteSlot::new("Test_2", Common, 246, "Action 2"),
        ];
        let slot_service = RouletteSlotService::new(slots);

        let total_weight = slot_service.total_weight();
        assert_eq!(total_weight, 369);
    }

    #[test]
    fn test_mid_boundary_switching() {
        let slots = vec![
            RouletteSlot::new("Test_1", Common, 10, "Action 1"),
            RouletteSlot::new("Test_2", Common, 20, "Action 2"),
        ];
        let slot_service = RouletteSlotService::new(slots);

        let slot = slot_service.get_slot_by_weight(0).unwrap();
        assert_eq!(slot.name, "Test_1".to_string());

        let slot = slot_service.get_slot_by_weight(9).unwrap();
        assert_eq!(slot.name, "Test_1".to_string());

        let slot = slot_service.get_slot_by_weight(10).unwrap();
        assert_eq!(slot.name, "Test_2".to_string());
    }

    #[test]
    fn test_absolute_favorite() {
        let slots = vec![
            RouletteSlot::new("Loser_1", Common, 0, "Loser 1"),
            RouletteSlot::new("Winner", Common, 20, "Winner"),
            RouletteSlot::new("Loser_2", Common, 0, "Loser 1"),
        ];
        let slot_service = RouletteSlotService::new(slots);

        let slot = slot_service.get_slot_by_weight(0).unwrap();
        assert_eq!(slot.name, "Winner".to_string());

        let slot = slot_service.get_slot_by_weight(10).unwrap();
        assert_eq!(slot.name, "Winner".to_string());

        let slot = slot_service.get_slot_by_weight(19).unwrap();
        assert_eq!(slot.name, "Winner".to_string());
    }

    #[test]
    fn test_fallback_heaviest() {
        let slots = vec![
            RouletteSlot::new("Loser_1", Common, 0, "Loser 1"),
            RouletteSlot::new("Winner", Common, 20, "Winner"),
            RouletteSlot::new("Loser_2", Common, 0, "Loser 1"),
        ];
        let slot_service = RouletteSlotService::new(slots);

        let slot = slot_service.get_slot_by_weight(30).unwrap();
        assert_eq!(slot.name, "Winner".to_string());
    }
}
