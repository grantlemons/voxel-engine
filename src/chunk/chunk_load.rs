use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};
use thiserror::Error;

use crate::{Location, block::Block, chunk::Biome, chunk::lazy_block::LazyBlock};

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("File not found!")]
    FileNotFound,
    #[error("Unable to load from file!")]
    InvalidLoad,
    #[error("Unable to roughen!")]
    InvalidRoughen,
    #[error("Unable to make more fine!")]
    InvalidFine,
}

pub static CHUNK_SIZE: usize = 10;
type Roughness = u8;
type LazyChunk<F> = [[[LazyBlock<F>; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

#[derive(Debug, Serialize, Deserialize)]
pub enum LoadState<F: Fn(&Location, &Biome) -> Block + Clone> {
    Ungenerated,
    StoredRough(PathBuf, Roughness),
    StoredFine(PathBuf),
    Rough(LazyChunk<F>, Roughness),
    Fine(LazyChunk<F>),
}

impl<F: Fn(&Location, &Biome) -> Block + Clone> LoadState<F> {
    pub fn generate_rough(self, roughness: Roughness) -> Result<Self, GenerationError> {
        match self {
            LoadState::Ungenerated => todo!(),
            LoadState::StoredRough(_, _) => self.load_stored()?.convert_rough(roughness),
            LoadState::StoredFine(_) => self.load_stored()?.convert_rough(roughness),
            LoadState::Rough(_, _) => self.convert_rough(roughness),
            LoadState::Fine(_) => self.convert_rough(roughness),
        }
    }

    pub fn generate_fine(self) -> Result<Self, GenerationError> {
        match self {
            LoadState::Ungenerated => self.convert_fine(),
            LoadState::StoredRough(_, _) => self.load_stored()?.convert_fine(),
            LoadState::StoredFine(_) => self.load_stored()?.convert_fine(),
            LoadState::Rough(_, _) => self.convert_fine(),
            LoadState::Fine(_) => Ok(self),
        }
    }

    fn convert_rough(self, roughness: Roughness) -> Result<Self, GenerationError> {
        match self {
            LoadState::Rough(_, r) if r == roughness => Ok(self),
            LoadState::Rough(mut src, _) | LoadState::Fine(mut src) => {
                let mid =
                    |a| (a / roughness as usize) * roughness as usize + (roughness as usize / 2);
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        for y in 0..CHUNK_SIZE {
                            src[z][x][y].overwrite_rough(src[mid(z)][mid(x)][mid(y)].force());
                        }
                    }
                }
                Ok(Self::Rough(src, roughness))
            }
            _ => Err(GenerationError::InvalidRoughen),
        }
    }

    fn convert_fine(self) -> Result<Self, GenerationError> {
        match self {
            Self::Rough(mut src, _) => {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        for y in 0..CHUNK_SIZE {
                            src[z][x][y].force_update();
                        }
                    }
                }
                Ok(Self::Fine(src))
            }
            Self::Fine(_) => Ok(self),
            _ => Err(GenerationError::InvalidFine),
        }
    }

    fn load_stored(&self) -> Result<Self, GenerationError> {
        match self {
            LoadState::StoredRough(_path_buf, _) => todo!(),
            LoadState::StoredFine(_path_buf) => todo!(),
            _ => Err(GenerationError::InvalidLoad),
        }
    }
}
