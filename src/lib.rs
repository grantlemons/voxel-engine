use glam::UVec3;

pub mod block;
pub mod chunk;
pub mod contree;
pub mod generation;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Material {
    color: [f32; 4],
    reflectivity: f32,
    padding: [u8; 12],
}

pub type ChunkLocation = UVec3;
pub type AbsoluteLocation = UVec3;
