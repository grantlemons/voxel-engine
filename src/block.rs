use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::LazyLock};
use tracing::{Level, span};

bitflags! {
    pub struct BlockTraits: u8 {
        const Empty = 0;
        const Flammable = 0b1;
    }
}

impl Default for &BlockTraits {
    fn default() -> Self {
        &BlockTraits::Empty
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Block {
    #[default]
    Air,
    Grass,
    Dirt,
    Wood,
}

impl Block {
    pub fn is_flammable(&self) -> bool {
        BLOCK_TRAITS
            .get(self)
            .unwrap_or_default()
            .contains(BlockTraits::Flammable)
    }
}

pub static BLOCK_TRAITS: LazyLock<HashMap<Block, BlockTraits>> = LazyLock::new(|| {
    span!(Level::TRACE, "Initializing block traits");
    HashMap::from([(Block::Wood, BlockTraits::Flammable)])
});
