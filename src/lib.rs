pub mod contree;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Material {
    color: [f32; 4],
    reflectivity: f32,
    padding: [u8; 12],
}
