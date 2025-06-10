struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) vel_mag: f32,
}
struct CameraUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
}
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}
 
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    let pos = instance.position;
    let model_matrix = mat4x4<f32>(
        vec4f(1.0,0.0,0.0,0.0),
        vec4f(0.0,1.0,0.0,0.0),
        vec4f(0.0,0.0,1.0,0.0),
        vec4f(pos.x,pos.y,pos.z,1.0)
    );

    var out: VertexOutput;
    let v = clamp(instance.vel_mag,0.0,1.0) / 1.0;
    out.color = vec4f(v,0.0,1.0 - v,1.0);
    // out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position =  camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
