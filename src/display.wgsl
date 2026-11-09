struct Emitter { position:vec4f,color:vec4f }
struct ReflectionCard {position:vec4f,appearance:vec4f}
struct Scene { viewport:vec4f,light:vec4f,environment:vec4f,probe:vec4f,effects:vec4f,emitters:array<Emitter,16>,weather:vec4f,camera:vec4f,cards:array<ReflectionCard,4>,world:vec4f }
@group(0) @binding(0) var<uniform> scene:Scene;
@group(1) @binding(0) var frame:texture_2d<f32>;
@group(1) @binding(1) var smooth_sampler:sampler;
@group(1) @binding(2) var bloom:texture_2d<f32>;
@vertex fn fullscreen(@builtin(vertex_index) i:u32)->@builtin(position)vec4f{
    let p=array<vec2f,3>(vec2f(-1.0,-1.0),vec2f(3.0,-1.0),vec2f(-1.0,3.0));return vec4f(p[i],0.0,1.0);
}
@fragment fn display(@builtin(position) position:vec4f)->@location(0)vec4f{
    var color=textureLoad(frame,vec2i(position.xy),0).rgb;
    // One display transform, after linear-light compositing. Debug/color modes bypass tone mapping.
    if scene.viewport.w<0.5 || scene.viewport.w>3.5{
        color+=textureSampleLevel(bloom,smooth_sampler,position.xy/scene.viewport.xy,0.0).rgb*scene.effects.x;
        color*=scene.environment.y;
        color=clamp((color*(2.51*color+0.03))/(color*(2.43*color+0.59)+0.14),vec3f(0.0),vec3f(1.0));
    }
    return vec4f(pow(max(color,vec3f(0.0)),vec3f(1.0/2.2)),1.0);
}
