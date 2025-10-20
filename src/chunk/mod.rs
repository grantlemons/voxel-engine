use serde::{Deserialize, Serialize};

use crate::{AbsoluteLocation, block::Block};

mod chunk_load;
mod lazy_block;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Biome {
    Plains,
    Forest,
    Mountain,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Chunk<F: Fn(&AbsoluteLocation, &Biome) -> Block + Clone + Send + Sync> {
    pub location: AbsoluteLocation,
    pub load_state: chunk_load::LoadState<F>,
    pub biome: Biome,
}
