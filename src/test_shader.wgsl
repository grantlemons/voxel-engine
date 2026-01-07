@group(0) @binding(0) var write_texture: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var read_texture: texture_storage_2d<rgba32float, read>;

@compute @workgroup_size(1, 1, 1)
fn cs_main(@builtin(local_invocation_id) id: vec3u) {
    let color = vec3f(1., 0., 0.);
    textureStore(write_texture, id.xy, vec4f(color, 1.));
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) xy: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let pos = array(
        vec2f(-1.0, 3.0),
        vec2f(3.0, -1.0),
        vec2f(-1.0, -1.0),
    );

    let xy = pos[index];
    return VertexOutput(vec4f(xy, 0.0, 1.0), xy);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureLoad(read_texture, vec2u(in.xy));
}
