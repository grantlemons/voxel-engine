struct ContreeLeaf {
    contains: array<u32, 2>,
    light: array<u32, 2>,
    children: array<u32, 16>,
}

struct ContreeInner {
    contains: array<u32, 2>,
    leaf: array<u32, 2>,
    light: array<u32, 2>,
    children: array<u32, 64>,
    default_material: u32,
}

struct Material {
    color: vec4f,
    reflectivity: f32,
}

var<push_constant> contree_root: u32;
var<push_constant> contree_center: f32;
var<push_constant> contree_size: vec3u;
@group(0) @binding(0) var<storage, read> inners: array<ContreeInner>;
@group(0) @binding(1) var<storage, read> leaves: array<ContreeLeaf>;
@group(0) @binding(2) var<storage, read> materials: array<Material>;

@compute @workgroup_size(1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
}

fn calculate_brightness(p: vec3f, dir: vec3f) -> vec3f {
    return vec3f(0.);
}

fn calculate_color(p: vec3f, dir: vec3f) -> vec3f {
    return vec3f(0.);
}

fn raytrace(p: vec3f, dir: vec3f) -> vec3f {
    return vec3f(0.);
}
