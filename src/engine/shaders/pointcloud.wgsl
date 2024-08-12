struct InstanceInput {
    @location(0) position: vec3<f32>,
};

struct Uniform {
    camera: mat4x4<f32>,
    resolution: vec2<f32>,
    size: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uni: Uniform;

@vertex fn vs_main(
    instance: InstanceInput,
    @builtin(vertex_index) vNdx: u32,
) -> VertexOutput {
    var points = array(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var out: VertexOutput;
    let pos = points[vNdx];
    let instance_pos = vec4<f32>(instance.position, 1.0);
    let clip_pos = uni.camera * instance_pos;
    let point_pos = vec4<f32>(pos * uni.size / uni.resolution * clip_pos.w, 0.0, 0.0);
    out.position = clip_pos + point_pos;
    return out;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}