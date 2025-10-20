use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{AbsoluteLocation, block::Block, generation::GenerationOutput};

#[derive(Deserialize, Serialize, Clone)]
pub enum LazyBlock<F: Fn(&AbsoluteLocation) -> GenerationOutput + Clone + Send + Sync> {
    Ungenerated(F, AbsoluteLocation),
    GeneratedRough(Block, Box<Self>),
    Generated(Block, Box<Self>),
}

impl<F: Fn(&AbsoluteLocation) -> GenerationOutput + Clone + Send + Sync> LazyBlock<F> {
    fn get_generated(&self) -> &Block {
        match self {
            Self::Ungenerated(..) => panic!("get_generated called on ungenerated block"),
            Self::GeneratedRough(block, ..) => block,
            Self::Generated(block, ..) => block,
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Ungenerated(..) => {}
            Self::GeneratedRough(_, ungen) => *self = *ungen.clone(),
            Self::Generated(_, ungen) => *self = *ungen.clone(),
        }
    }

    pub fn overwrite_rough(&mut self, block: Block) {
        match self {
            Self::Ungenerated(..) => {
                *self = Self::GeneratedRough(block, Box::new(self.clone()));
            }
            Self::GeneratedRough(_, ungen) => *self = Self::GeneratedRough(block, ungen.clone()),
            Self::Generated(_, ungen) => *self = Self::GeneratedRough(block, ungen.clone()),
        }
    }

    pub fn force_update(&mut self) -> &Block {
        match self {
            Self::Ungenerated(f, location) => {
                let res = f(location);
                *self = Self::Generated(res.block, Box::new(self.clone()));
                self.get_generated()
            }
            Self::GeneratedRough(..) => {
                self.reset();
                self.force_update()
            }
            Self::Generated(block, ..) => block,
        }
    }

    pub fn force(&self) -> Block {
        match self {
            Self::Ungenerated(f, location) => f(location).block,
            Self::GeneratedRough(block, ..) => *block,
            Self::Generated(block, ..) => *block,
        }
    }
}

impl<F: Fn(&AbsoluteLocation) -> GenerationOutput + Clone + Send + Sync> Debug for LazyBlock<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ungenerated(_, loc) => f.debug_tuple("Ungenerated").field(loc).finish(),
            Self::GeneratedRough(block, ..) => {
                f.debug_tuple("Generated Rough").field(block).finish()
            }
            Self::Generated(block, ..) => f.debug_tuple("Generated").field(block).finish(),
        }
    }
}
