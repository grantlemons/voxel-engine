@group(0) @binding(0) var out_texture: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var in_texture: texture_storage_2d<rgba32float, read>;

@compute @workgroup_size(1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let color = vec3f(1., 1., 0.);
    textureStore(out_texture, id.xy, vec4f(color, 1.));
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) xy: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
  let pos = array(
    // 1st triangle
    vec2f( 0.0,  0.0),  // center
    vec2f( 1.0,  0.0),  // right, center
    vec2f( 0.0,  1.0),  // center, top
 
    // 2nd triangle
    vec2f( 0.0,  1.0),  // center, top
    vec2f( 1.0,  0.0),  // right, center
    vec2f( 1.0,  1.0),  // right, top
  );

  let xy = pos[index];
  return VertexOutput(vec4f(xy, 0.0, 1.0), xy);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
  return textureLoad(in_texture, vec2u(in.xy));
}
