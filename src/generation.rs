// use serde::{Deserialize, Serialize};

use crate::block::Block;

#[derive(Clone, Copy, Default, Debug)]
pub struct GenerationOutput {
    pub block: Block,
}
