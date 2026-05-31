use bytemuck::{Pod, Zeroable};

mod datastructure;
mod gpu_binding;
mod util;

pub use datastructure::Contree;

// 80 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeLeaf {
    contains: u64,
    light: u64,
    children: [u8; 64],
}

// 280 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeInner {
    contains: u64,
    leaf: u64,
    light: u64,
    children: [Addr; 64],
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Material {
    color: [f32; 4],
    reflectivity: f32,
    padding: [u8; 12],
}

type ChildIndex = usize;

/// Address in terms of data type, not bytes
type Addr = u32;

#[derive(Debug)]
pub struct FindResult {
    leaf_address: Option<Addr>,
    traversal_stack: Vec<ChildIndex>,
    parent_addrs: Vec<Addr>,
    /// Distance from face to face
    node_size: u32,
}
