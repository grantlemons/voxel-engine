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
}

struct Material {
    color: vec4f,
    reflectivity: f32,
}

var<push_constant> contree_root: u32;
var<push_constant> contree_center: f32;
var<push_constant> contree_size: u32;
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


fn interleave(val: u32) -> array<u32, 2> {
    var magic_numbers = array<array<u32, 2>, 5>(
        array(0x001f0000u, 0x0000ffffu),
        array(0x001f0000u, 0xff0000ffu),
        array(0x100f00ffu, 0x0f00f00fu),
        array(0x10c30c30u, 0xc30c30c3u),
        array(0x12492492u, 0x49249249u),
    );

    // only the first 21 bits are used
    var x = array<u32, 2>(0u, val & 0x1fffffu);

    x[0] = x[1] & magic_numbers[0][0];
    x[1] = x[1] & magic_numbers[0][1];

    x[0] = (x[0] | (x[0] << 16u) | (x[1] >> 16u)) & magic_numbers[1][0];
    x[1] = (x[1] | (x[1] << 16u)) & magic_numbers[1][1];

    x[0] = (x[0] | (x[0] << 8u) | (x[1] >> 24u)) & magic_numbers[2][0];
    x[1] = (x[1] | (x[1] << 8u)) & magic_numbers[2][1];

    x[0] = (x[0] | (x[0] << 4u) | (x[1] >> 28u)) & magic_numbers[3][0];
    x[1] = (x[1] | (x[1] << 4u)) & magic_numbers[3][1];

    x[0] = (x[0] | (x[0] << 2u) | (x[1] >> 30u)) & magic_numbers[4][0];
    x[1] = (x[1] | (x[1] << 2u)) & magic_numbers[4][1];

    return x;
}

fn morton_code(p: vec3u) -> array<u32, 2> {
    let x_interleaved = interleave(p.x);
    let x_upper = ((x_interleaved[0] << 2u) | (x_interleaved[1] >> 30u));

    let y_interleaved = interleave(p.y);
    let y_upper = ((x_interleaved[0] << 1u) | (x_interleaved[1] >> 31u));

    let z_interleaved = interleave(p.z);

    return array<u32, 2>(x_upper | y_upper | z_interleaved[0], x_interleaved[1] | x_interleaved[1] | z_interleaved[1]);
}

// check if digit >= 64 to only get valid codes
fn to_base_64(code: array<u32, 2>) -> array<u32, 10> {
    var digits = array<u32, 10>(64, 64, 64, 64, 64, 64, 64, 64, 64, 64);
    var i = 0;
    var n = code;
    if n[0] == 0u && n[1] == 0u {
        digits[0] = 0u;
        return digits;
    }
    while n[0] != 0u || n[1] != 0u {
        digits[i] = n[1] & 0x3fu;

        n[1] = (n[0] << 26u) | (n[1] >> 6u);
        n[0] >>= 6u;
    }

    return digits;
}
