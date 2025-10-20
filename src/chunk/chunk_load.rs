use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};
use thiserror::Error;

use crate::{
    AbsoluteLocation, ChunkLocation,
    block::Block,
    chunk::{Biome, lazy_block::LazyBlock},
};

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("File not found!")]
    FileNotFound,
    #[error("Unable to load from file!")]
    InvalidLoad,
}

/// Chunk size in each dimension
pub static CHUNK_SIZE: usize = 12;
/// Roughness is an integer divisor of [CHUNK_SIZE]
type Detail = u8;
/// Static array of lazy blocks
type LazyChunk<F> = [RwLock<[[LazyBlock<F>; CHUNK_SIZE]; CHUNK_SIZE]>; CHUNK_SIZE];

#[derive(Serialize, Deserialize, Debug)]
pub enum LoadState<F: Fn(&AbsoluteLocation, &Biome) -> Block + Clone + Send + Sync> {
    Ungenerated(LazyChunk<F>),
    StoredRough(PathBuf, Detail),
    StoredFine(PathBuf),
    Rough(LazyChunk<F>, Detail),
    Fine(LazyChunk<F>),
}

pub fn lazy_chunk<F: Fn(&AbsoluteLocation, &Biome) -> Block + Clone + Send + Sync>(
    f: F,
    chunk_location: ChunkLocation,
    biome: Biome,
) -> LazyChunk<F> {
    let val = |z, x, y| {
        LazyBlock::Ungenerated(
            f.clone(),
            AbsoluteLocation::new(x as u32, y as u32, z as u32) + chunk_location,
            biome,
        )
    };
    let inner = |z, x| {
        let partial = |y| val(z, x, y);
        std::array::from_fn::<_, CHUNK_SIZE, _>(partial)
    };
    let outer = |z| {
        let partial = |x| inner(z, x);
        RwLock::new(std::array::from_fn::<_, CHUNK_SIZE, _>(partial))
    };
    std::array::from_fn::<_, CHUNK_SIZE, _>(outer)
}

impl<F: Fn(&AbsoluteLocation, &Biome) -> Block + Clone + Send + Sync> LoadState<F> {
    pub fn new(f: F, chunk_location: ChunkLocation, biome: Biome) -> Self {
        Self::Ungenerated(lazy_chunk(f, chunk_location, biome))
    }

    pub fn rough(self, detail: Detail) -> Result<Self, GenerationError> {
        match self {
            Self::Rough(_, r) if r == detail => Ok(self),
            Self::Ungenerated(src) | Self::Rough(src, _) | Self::Fine(src) => {
                let division_size = CHUNK_SIZE / detail as usize;
                let mid = |a: usize| {
                    (a / division_size) * division_size as usize + (division_size as usize / 2)
                };
                let dim_pariter = (0..CHUNK_SIZE).into_par_iter();
                dim_pariter.clone().for_each(|z| {
                    (0..CHUNK_SIZE).for_each(|x| {
                        (0..CHUNK_SIZE).for_each(|y| {
                            if (z, x, y) == (mid(z), mid(x), mid(y)) {
                                src[z].write()[x][y].force_update();
                            } else {
                                let mid_value = { src[mid(z)].read()[mid(x)][mid(y)].force() };
                                src[z].write()[x][y].overwrite_rough(mid_value);
                            }
                        })
                    })
                });
                Ok(Self::Rough(src, detail))
            }
            Self::StoredRough(..) => self.load_stored()?.rough(detail),
            Self::StoredFine(..) => self.load_stored()?.rough(detail),
        }
    }

    pub fn fine(self) -> Result<Self, GenerationError> {
        match self {
            Self::Ungenerated(src) | Self::Rough(src, _) => {
                let dim_pariter = (0..CHUNK_SIZE).into_par_iter();
                dim_pariter.clone().for_each(|z| {
                    let mut z_layer = src[z].write();
                    (0..CHUNK_SIZE).for_each(|x| {
                        (0..CHUNK_SIZE).for_each(|y| {
                            z_layer[x][y].force_update();
                        })
                    })
                });
                Ok(Self::Fine(src))
            }
            Self::Fine(_) => Ok(self),
            Self::StoredRough(..) => self.load_stored()?.fine(),
            Self::StoredFine(..) => self.load_stored()?.fine(),
        }
    }

    fn load_stored(&self) -> Result<Self, GenerationError> {
        match self {
            Self::StoredRough(_path_buf, _) => todo!(),
            Self::StoredFine(_path_buf) => todo!(),
            _ => Err(GenerationError::InvalidLoad),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AbsoluteLocation,
        block::Block,
        chunk::{Biome, chunk_load::LoadState},
    };

    #[test]
    fn test_rough() {
        let state = LoadState::new(
            |_, _| Block::Wood,
            AbsoluteLocation::default(),
            Biome::Forest,
        )
        .rough(2);
        assert!(matches!(state, Ok(LoadState::Rough(_, 2))));

        match state.unwrap() {
            LoadState::Rough(src, _) => {
                dbg!(&src[3].read()[3]);
            }
            _ => {}
        }
    }

    #[test]
    fn test_fine() {
        let state = LoadState::new(
            |_, _| Block::Wood,
            AbsoluteLocation::default(),
            Biome::Forest,
        )
        .fine();
        assert!(matches!(state, Ok(LoadState::Fine(_))));

        match state.unwrap() {
            LoadState::Fine(src) => {
                dbg!(&src[3].read()[3]);
            }
            _ => {}
        }
    }

    #[test]
    fn test_rough_to_fine() {
        let mut state = LoadState::new(
            |_, _| Block::Wood,
            AbsoluteLocation::default(),
            Biome::Forest,
        )
        .rough(2);
        assert!(matches!(state, Ok(LoadState::Rough(_, 2))));
        state = state.unwrap().fine();
        assert!(matches!(state, Ok(LoadState::Fine(_))));

        match state.unwrap() {
            LoadState::Fine(src) => {
                dbg!(&src[3].read()[3]);
            }
            _ => {}
        }
    }
}
