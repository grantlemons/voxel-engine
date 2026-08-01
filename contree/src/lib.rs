use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

mod finding;
mod gpu_binding;
mod node_insertion;
mod node_management;
mod raycasting;
mod util;

use glam::Vec3;

pub use gpu_binding::*;

// 80 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Serialize, Deserialize)]
pub struct ContreeLeaf {
    pub contains: u64,
    pub light: u64,
    #[serde(with = "serde_arrays")]
    pub children: [u8; 64],
}

// 280 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Serialize, Deserialize)]
pub struct ContreeInner {
    pub contains: u64,
    pub leaf: u64,
    pub light: u64,
    #[serde(with = "serde_arrays")]
    pub children: [Addr; 64],
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct Material {
    pub color: [f32; 4],
    pub reflectivity: f32,
    pub padding: [u8; 12],
}

type ChildIndex = u8;

/// Address in terms of data type, not bytes
/// Byte address = Addr * sizeof(node)
pub type Addr = u32;

#[derive(Debug)]
pub struct Contree {
    pub center_offset: Vec3,
    pub root: Option<Addr>,
    /// Distance from face to face
    pub size: u32,
    pub inners: Vec<ContreeInner>,
    pub leaves: Vec<ContreeLeaf>,
    pub inner_tombstones: Vec<Addr>,
    pub leaf_tombstones: Vec<Addr>,
    pub binding: Box<dyn GPUBindable>,
}

impl Default for Contree {
    fn default() -> Self {
        Self::new(Box::new(DummyBinding))
    }
}

impl Contree {
    pub fn new(binding: Box<dyn GPUBindable>) -> Self {
        let mut new = Self {
            center_offset: Default::default(),
            root: None,
            size: 16,
            inners: Default::default(),
            leaves: Default::default(),
            inner_tombstones: Default::default(),
            leaf_tombstones: Default::default(),
            binding,
        };
        new.root = Some(new.create_root_node());
        new
    }
}

impl std::fmt::Display for Contree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(root) = self.root {
            let mut stack = vec![root];

            writeln!(
                f,
                "digraph {{
\tnewrank=true;
\trankdir=LR;"
            )?;
            while let Some(addr) = stack.pop() {
                let cur = self.inners[addr as usize];
                for i in 0..64 {
                    if (cur.contains & (0b1 << i)) != 0 {
                        if (cur.leaf & (0b1 << i)) != 0 {
                            writeln!(
                                f,
                                "\t{} -> \"leaf {}\" [label=<{}>]",
                                addr, cur.children[i], i
                            )?;

                            let leaf_addr = cur.children[i];
                            for j in 0..64 {
                                if (self.leaves[leaf_addr as usize].contains & (0b1 << j)) != 0 {
                                    writeln!(
                                        f,
                                        "\t\"leaf {}\" -> \"mat {}\" [label=<{}>]",
                                        leaf_addr, self.leaves[leaf_addr as usize].children[j], j
                                    )?;
                                }
                            }
                        } else {
                            writeln!(f, "\t{} -> {} [label=<{}>]", addr, cur.children[i], i)?;
                            stack.push(cur.children[i]);
                        }
                    }
                }
            }

            writeln!(f, "}}")
        } else {
            writeln!(f, "Empty contree")
        }
    }
}
