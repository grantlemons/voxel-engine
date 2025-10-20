use serde::{Deserialize, Serialize};

use crate::{Location, block::Block};

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
pub struct Chunk<F: Fn(&Location, &Biome) -> Block + Clone> {
    /// Outer array is Z
    /// Next is X
    /// Next is Y
    /// All in all blocks[Z][X][Y]
    pub load_state: chunk_load::LoadState<F>,
    pub biome: Biome,
}
