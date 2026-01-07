@group(0) @binding(0) var out_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let color = vec3f(0., 0., 0.);
    textureStore(out_texture, id.xy, vec4f(color, 1.));
}
