use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{Location, block::Block, chunk::Biome};

#[derive(Deserialize, Serialize, Clone)]
pub enum LazyBlock<F: Fn(&Location, &Biome) -> Block + Clone> {
    Ungenerated(F, Location, Biome),
    GeneratedRough(Block, Box<Self>),
    Generated(Block, Box<Self>),
}

impl<F: Fn(&Location, &Biome) -> Block + Clone> LazyBlock<F> {
    fn get_generated(&self) -> &Block {
        match self {
            Self::Ungenerated(..) => panic!("get_generated called on ungenerated block"),
            Self::GeneratedRough(block, _) => block,
            Self::Generated(block, _) => block,
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
            Self::Ungenerated(f, location, biome) => {
                let res = f(location, biome);
                *self = Self::Generated(res, Box::new(self.clone()));
                self.get_generated()
            }
            Self::GeneratedRough(block, ..) => block,
            Self::Generated(block, ..) => block,
        }
    }

    pub fn force(&self) -> Block {
        match self {
            Self::Ungenerated(f, location, biome) => f(location, biome),
            Self::GeneratedRough(block, ..) => *block,
            Self::Generated(block, ..) => *block,
        }
    }
}

impl<F: Fn(&Location, &Biome) -> Block + Clone> Debug for LazyBlock<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ungenerated(_, loc, biome) => f
                .debug_tuple("Ungenerated")
                .field(loc)
                .field(biome)
                .finish(),
            Self::GeneratedRough(block, ..) => {
                f.debug_tuple("Generated Rough").field(block).finish()
            }
            Self::Generated(block, ..) => f.debug_tuple("Generated").field(block).finish(),
        }
    }
}
