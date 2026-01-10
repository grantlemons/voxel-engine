struct Camera {
    position: vec3f,
    rotation: vec3f, // pitch, yaw, roll
    fov: f32,
    size: vec2u,
};

var<push_constant> camera: Camera;

fn euclidian_mod(a: vec2f, b: vec2f) -> vec2f {
    return a - floor(a / b) * b;
}

fn rotation_matrix(rad_rot: vec3f) -> mat3x3f {
    // column-wise constructor
    return mat3x3f(
        vec3f(
            cos(rad_rot.y) * cos(rad_rot.z),
            cos(rad_rot.y) * sin(rad_rot.z),
            -sin(rad_rot.y),
        ),
        vec3f(
            sin(rad_rot.x) * sin(rad_rot.y) * cos(rad_rot.z)
                - cos(rad_rot.x) * sin(rad_rot.z),
            sin(rad_rot.x) * sin(rad_rot.y) * sin(rad_rot.z)
                + cos(rad_rot.x) * cos(rad_rot.z),
            sin(rad_rot.x) * cos(rad_rot.y),
        ),
        vec3f(
            cos(rad_rot.x) * sin(rad_rot.y) * cos(rad_rot.z)
                + sin(rad_rot.x) * sin(rad_rot.z),
            cos(rad_rot.x) * sin(rad_rot.y) * sin(rad_rot.z)
                - sin(rad_rot.x) * cos(rad_rot.z),
            cos(rad_rot.x) * cos(rad_rot.y),
        ),
    );
}

const LIGHT_POS: vec3f = vec3f(-8., 7., 2.);
const LIGHT_BRIGHTNESS: f32 = 100.;
const SPHERE_POS: vec3f = vec3f(0., 0., 10.);
const SPHERE_RADIUS: f32 = 5.;
fn sphere_dist(pos: vec3f) -> f32 {
    return length(SPHERE_POS - pos) - SPHERE_RADIUS;
}

fn floor_dist(pos: vec3f) -> f32 {
    return pos.y + 5.;
}

fn light_dist(pos: vec3f) -> f32 {
    return length(LIGHT_POS - pos);
}

struct RayHit {
    pos: vec3f,
    hit: bool,
}

fn map(pos: vec3f) -> f32 {
    return min(floor_dist(pos), sphere_dist(pos));
}

fn raymarch_intersect(pos: vec3f, dir: vec3f) -> RayHit {
    var t = 0.;

    while (t <= 500.) {
        let dist = map(pos + t * dir);

        if (dist <= 0.01) {
            return RayHit(pos + t * dir, true);
        } else {
            t = t + dist;
        }
    }

    return RayHit(vec3f(0.), false);
}

fn compute_brightness(pos: vec3f) -> f32 {
    let between = LIGHT_POS - pos;
    let dir = normalize(between);

    var t = 0.;

    while (t <= 50.) {
        let dist = min(map(pos + t * dir), light_dist(pos + t * dir));

        if (dist <= 0.001) {
            return LIGHT_BRIGHTNESS/(4 * 3.141592 * pow(length(between), 2));
        } else {
            t = t + dist;
        }
    }

    return 0;
}

fn calculate_ray_color(pos: vec3f, dir: vec3f) -> vec3f {
    let raymarch = raymarch_intersect(pos, dir);

    if raymarch.hit {
        return vec3f(compute_brightness(raymarch.pos));
    }

    return vec3f(0.);
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let pos = array(
        vec2f(-1.0, 3.0),
        vec2f(3.0, -1.0),
        vec2f(-1.0, -1.0),
    );

    return vec4f(pos[index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) in: vec4f) -> @location(0) vec4f {
    let id = vec2u(in.yx);
    let size = vec2f(camera.size);

    let near_dist = 957.0;
    //f32(size.x)/(2 * tan(radians(camera.fov/2)));
    let pixel_vec = vec3f(f32(id.x) - (size.x/2.), f32(id.y) - (size.y/2.), near_dist);
    let dir = normalize(rotation_matrix(radians(-camera.rotation)) * pixel_vec);

    let color = calculate_ray_color(camera.position, dir);

    return vec4f(color, 1.);
}
