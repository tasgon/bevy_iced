
struct VertexOutput {
    [[location(0)]] tex_coord: vec2<f32>;
    [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] index: u32
) -> VertexOutput {
    var tex_coords: array<vec2<f32>,4u> = array<vec2<f32>,4u>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    var positions: array<vec2<f32>,4u> = array<vec2<f32>,4u>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    let idx = i32(index);
    let pos = positions[idx];
    out.pos = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    out.tex_coord = tex_coords[idx];
    return out;
}

[[group(0), binding(0)]]
var tex_iced: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler_iced: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(tex_iced, sampler_iced, in.tex_coord);
}