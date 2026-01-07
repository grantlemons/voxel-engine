@group(0) @binding(0) var write_texture: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var read_texture: texture_storage_2d<rgba32float, read>;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(write_texture);
    let color = vec3f(vec2f(id.xy) / vec2f(size), 0.);
    textureStore(write_texture, id.xy, vec4f(color, 1.));
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
    return textureLoad(read_texture, vec2u(in.xy));
}
