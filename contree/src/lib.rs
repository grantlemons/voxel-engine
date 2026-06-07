use bytemuck::{Pod, Zeroable};

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
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ContreeLeaf {
    contains: u64,
    light: u64,
    children: [u8; 64],
}

// 280 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ContreeInner {
    contains: u64,
    leaf: u64,
    light: u64,
    children: [Addr; 64],
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    color: [f32; 4],
    reflectivity: f32,
    padding: [u8; 12],
}

type ChildIndex = usize;

/// Address in terms of data type, not bytes
/// Byte address = Addr * sizeof(node)
pub type Addr = u32;

#[derive(Debug)]
pub struct Contree {
    pub center_offset: Vec3,
    pub root: Addr,
    /// Distance from face to face
    pub size: u32,
    inners: Vec<ContreeInner>,
    leaves: Vec<ContreeLeaf>,
    inner_tombstones: Vec<Addr>,
    leaf_tombstones: Vec<Addr>,
    binding: Box<dyn GPUBindable>,
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
            root: Default::default(),
            size: 16,
            inners: Default::default(),
            leaves: Default::default(),
            inner_tombstones: Default::default(),
            leaf_tombstones: Default::default(),
            binding,
        };
        new.root = new.create_root_node();
        new
    }
}

impl std::fmt::Display for Contree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "digraph {{
\tnewrank=true;
\trankdir=LR;"
        )?;

        let mut stack = vec![self.root];

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
    }
}
