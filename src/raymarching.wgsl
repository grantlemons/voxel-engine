struct SceneProperties {
    size: vec2u,
    camera_rot: vec2f,
    camera_pos: vec3f,
};

var<push_constant> properties: SceneProperties;

// output of a vertex shader
struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let aspect_ratio = f32(properties.size.x) / f32(properties.size.y);
    let uv = vec2f(in.uv.x, in.uv.y * aspect_ratio);

    let cos_yaw = cos(properties.camera_rot.x);
    let sin_yaw = cos(properties.camera_rot.x);
    let cos_pitch = cos(properties.camera_rot.y);
    let sin_pitch = cos(properties.camera_rot.y);

    let rotation_matrix_x = mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, cos_pitch, sin_pitch,
        0.0, -sin_pitch, cos_pitch
    );
    let rotation_matrix_y = mat3x3<f32>(
        cos_yaw, 0.0, -sin_yaw,
        0.0, 1.0, 0.0,
        sin_yaw, 0.0, cos_yaw
    );

    let camera_dir = normalize(rotation_matrix_x * rotation_matrix_y * vec3f(uv, 1.0));
    let color = raymarch(properties.camera_pos, camera_dir);
    let tone_mapped = color / (color + 1.);

    return vec4f(tone_mapped, 1.);
}

struct Voxel {
    location: vec3f,
    // distance from center to edge of box in each dimension
    dims: vec3f,
    color: vec3f,
}

@group(0) @binding(0) var<storage, read> voxels: array<Voxel>;
@group(0) @binding(1) var<storage, read> lights: array<Voxel>;
var<push_constant> ambient_light: vec3f;

fn max_elem(v: vec3f) -> f32 {
    return max(v.x, max(v.y, v.z));
}

fn sd_voxel(p: vec3f, voxel: Voxel) -> f32 {
    let q = abs(p - voxel.location) - voxel.dims;
    return length(max(q, vec3f(0.0))) + min(max_elem(q), 0.0);
}

const EPSILON = 0.0001;
fn voxel_normal(p: vec3f, voxel: Voxel) -> vec3f {
    // find rough normal via gradient using the "Tetrahedron technique"
    let k = vec2f(1., -1.);
    return normalize(
        k.xyy * sd_voxel(p + k.xyy * EPSILON, voxel) + k.yyx * sd_voxel(p + k.yyx * EPSILON, voxel) + k.yxy * sd_voxel(p + k.yxy * EPSILON, voxel) + k.xxx * sd_voxel(p + k.xxx * EPSILON, voxel)
    );
}

struct ClosestVoxel {
    voxel: Voxel,
    distance: f32,
}

fn closest_voxel(p: vec3f) -> ClosestVoxel {
    let num_voxels = arrayLength(&voxels);

    var min_voxel: Voxel = voxels[0];
    var min_distance: f32 = sd_voxel(p, min_voxel);
    for (var i: u32 = 1u; i < num_voxels; i++) {
        let current_voxel = voxels[i];
        let d = sd_voxel(p, current_voxel);
        if d < min_distance {
            min_voxel = current_voxel;
            min_distance = d;
        }
    }

    return ClosestVoxel(min_voxel, min_distance);
}

const SHADOW_MAX_STEPS: u32 = 256u;
const SHADOW_MIN_DISTANCE: f32 = 0.;
// w is softness of shadow
fn light_brightness(p: vec3f, light_dir: vec3f, w: f32, max_distance: f32) -> f32 {
    var res: f32 = 1.;
    var t: f32 = SHADOW_MIN_DISTANCE;

    for (var i = 0u; i < SHADOW_MAX_STEPS && t < max_distance; i++) {
        let h = closest_voxel(p + t * light_dir).distance;
        res = min(res, h / (w * t));
        t += clamp(h, 0.005, 0.50);
        if res < -1.0 || t > SHADOW_MIN_DISTANCE {
            break;
        }
    }

    res = max(res, -1.0);
    return 0.25 * (1.0 + res) * (1.0 + res) * (2.0 - res);
}

fn calculate_color(p: vec3f, norm: vec3f, material_color: vec3f) -> vec3f {
    var light_color = vec3f(0.);

    for (var i = 0u; i < arrayLength(&lights); i++) {
        let light_pos = lights[i].location;
        let light_dir = normalize(light_pos - p);
        let light_dist = length(light_pos - p);

        let diffuse: f32 = max(dot(norm, light_dir), 0.);
        let brightness: f32 = light_brightness(p, light_dir, 16., light_dist);
        light_color += lights[i].color * (diffuse * brightness);
    }

    return material_color * light_color;
}

const MAX_STEPS: u32 = 512u;
const MAX_DISTANCE: f32 = 10000.;
const MAX_DISTANCE_REFLECTION: f32 = 1000.;
const MIN_DISTANCE: f32 = 0.;
const DIST_THRESHOLD: f32 = 0.0001;
const MAX_BOUNCES: u32 = 3u;
fn raymarch(start_p: vec3f, start_dir: vec3f) -> vec3f {
    var p: vec3f = start_p;
    var dir: vec3f = normalize(start_dir);
    var distance = MIN_DISTANCE;
    var bounces = 0u;

    var color = vec3f(0.);
    for (var steps: u32 = 0u; steps <= MAX_STEPS && distance <= select(MAX_DISTANCE, MAX_DISTANCE_REFLECTION, bounces > 0u) && bounces < MAX_BOUNCES; steps++) {
        p += dir * distance;

        let closest = closest_voxel(p);
        if abs(closest.distance) < DIST_THRESHOLD {
            let normal = voxel_normal(p, closest.voxel);
            color += calculate_color(p, normal, closest.voxel.color);

            dir = reflect(p, normal);
            bounces++;
        }

        distance = closest.distance;
    }

    return color;
}
