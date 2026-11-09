// A separable, quarter-resolution HDR bright pass. Scene pixels stay nearest-sampled.
@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var smooth_sampler:sampler;
struct Screen { @builtin(position) position:vec4f, @location(0) uv:vec2f }
@vertex fn fullscreen(@builtin(vertex_index) i:u32)->Screen {
    let p=array<vec2f,3>(vec2f(-1.0,-1.0),vec2f(3.0,-1.0),vec2f(-1.0,3.0));
    var o:Screen; o.position=vec4f(p[i],0.0,1.0); o.uv=vec2f(p[i].x*0.5+0.5,0.5-p[i].y*0.5); return o;
}
@fragment fn horizontal(i:Screen)->@location(0)vec4f {
    let texel=1.0/vec2f(textureDimensions(source));
    var c=vec3f(0.0); var weight=0.0;
    for(var j=-8;j<=8;j++) {
        let w=exp(-f32(j*j)/24.0);
        // Four taps retain small luminous pixels during the 4x downsample.
        var bright=vec3f(0.0);
        for(var k=0;k<4;k++) {
            let delta=vec2f(f32(j)*4.0+select(-1.0,1.0,k%2==1),select(-1.0,1.0,k>=2))*texel;
            let s=textureSampleLevel(source,smooth_sampler,i.uv+delta,0.0).rgb;
            let peak=max(s.r,max(s.g,s.b));
            bright+=s*max(peak-0.8,0.0)/max(peak,0.001)*0.25;
        }
        c+=bright*w; weight+=w;
    }
    return vec4f(c/weight,1.0);
}
@fragment fn vertical(i:Screen)->@location(0)vec4f {
    let texel=1.0/vec2f(textureDimensions(source));
    var c=vec3f(0.0); var weight=0.0;
    for(var j=-8;j<=8;j++) {
        let w=exp(-f32(j*j)/24.0);
        c+=textureSampleLevel(source,smooth_sampler,i.uv+vec2f(0.0,f32(j))*texel,0.0).rgb*w; weight+=w;
    }
    return vec4f(c/weight,1.0);
}
