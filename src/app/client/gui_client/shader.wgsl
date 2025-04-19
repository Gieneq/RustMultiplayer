// Vertex input structure matching the Vertex struct in Rust.
struct VertexInput {
    @location(0) position: vec4<f32>,
};

// The uniform structure with our color.
struct Uniforms {
    color: vec4<f32>,
};

// Bind the uniform to group 0, binding 0.
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    // Pass through the vertex position.
    return input.position;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Output a solid red color.
    // return vec4<f32>(0.14, 0.14, 0.14, 1.0);
    // Use the uniform color for output.
    return uniforms.color;
}

// @vertex
// fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
//     let x = f32(i32(in_vertex_index) - 1) / 2.0;
//     let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
//     return vec4<f32>(x, y, 0.0, 1.0);
// }

// @fragment
// fn fs_main() -> @location(0) vec4<f32> {
//     return vec4<f32>(1.0, 0.0, 0.0, 1.0);
// }
