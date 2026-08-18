use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct ActionId(u32);

impl ActionId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
