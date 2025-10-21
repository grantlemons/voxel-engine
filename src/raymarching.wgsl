struct Voxel {
    location: vec3f,
    color: vec3f,
}

@group(0) @binding(0) var<storage, read> voxels: array<Voxel>;
var<push_constant> light_pos: vec3f;

fn max_elem(v: vec3f) -> f32 {
    return max(v.x, max(v.y, v.z));
}

fn sd_voxel(p: vec3f, voxel: Voxel) -> f32 {
    let q = abs(p) - voxel.location;
    return length(max(q, vec3f(0.0))) + min(max_elem(q), 0.0);
}

struct ClosestVoxel {
    index: u32,
    distance: f32,
}

fn closest_voxel(p: vec3f) -> ClosestVoxel {
    let num_voxels = arrayLength(&voxels);

    var min_distance: f32 = sd_voxel(p, voxels[0]);
    var min_index: u32 = 0u;
    for (var i: u32 = 1u; i < num_voxels; i++) {
        let d = sd_voxel(p, voxels[i]);
        if d < min_distance {
            min_distance = d;
            min_index = i;
        }
    }

    return ClosestVoxel(min_index, min_distance);
}

const SHADOW_MAX_STEPS: u32 = 32u;
const SHADOW_MAX_DISTANCE: f32 = 10000.;
const SHADOW_DIST_THRESHOLD: f32 = 0.1;
fn raymarch_shadows(start_p: vec3f, start_dir: vec3f, softness: f32) -> f32 {
    var p: vec3f = start_p;
    var dir: vec3f = normalize(start_dir);
    var res = 1.;
    var t = 0.2;

    var closest = closest_voxel(p + dir * t);
    for (var steps: u32 = 0u; steps <= SHADOW_MAX_STEPS && closest.distance <= SHADOW_MAX_DISTANCE; steps++) {
        p = p + dir * closest.distance;
        closest = closest_voxel(p + dir * t);

        if abs(closest.distance) < SHADOW_DIST_THRESHOLD {
            return 0.;
        }

        res = min(res, softness * closest.distance / t);
        t += closest.distance;
    }

    return res;
}

const AMBIENT_LIGHT: f32 = 0.1;
fn calculate_light(p: vec3f, norm: vec3f, color: vec3f) -> vec3f {
    let light_dir = normalize(light_pos - p);
    let light_dist = length(light_pos - p);

    let diffuse = max(dot(norm, light_dir), 0.);
    let shadow = raymarch_shadows(p, light_dir, 16.);

    return color * (AMBIENT_LIGHT + diffuse * shadow);
}

struct Hit {
    hit: bool,
    color: vec3f,
}

const MAX_STEPS: u32 = 512u;
const MAX_DISTANCE: f32 = 10000.;
const DIST_THRESHOLD: f32 = 0.001;
fn raymarch(start_p: vec3f, start_dir: vec3f) -> Hit {
    var p: vec3f = start_p;
    var dir: vec3f = normalize(start_dir);
    var closest = closest_voxel(p);

    var res = Hit(false, vec3f(0.));
    for (var steps: u32 = 0u; steps <= MAX_STEPS && closest.distance <= MAX_DISTANCE; steps++) {
        p = p + dir * closest.distance;
        closest = closest_voxel(p);

        if abs(closest.distance) < DIST_THRESHOLD {
            res.hit = true;
            res.color += calculate_light(p, vec3f(0.), voxels[closest.index].color);
            break;
        }
    }

    return res;
}
