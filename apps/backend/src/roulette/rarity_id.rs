#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RarityId(u32);

impl RarityId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub(crate) const fn value(&self) -> u32 {
        self.0
    }
}
