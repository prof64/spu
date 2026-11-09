// Screen-space radiance/depth and tiled forward lighting. One bounce; no hidden views.
@group(2) @binding(1) var world_color:texture_2d<f32>;
@group(2) @binding(2) var world_geometry:texture_2d<f32>;
@group(2) @binding(3) var<storage,read> world_lights:array<Emitter>;
@group(2) @binding(4) var<storage,read> light_tiles:array<u32>;
fn world_position(screen:vec2f,entity:vec4f)->vec3f {
    let foot=select(entity.y/scene.viewport.y,screen.y,entity.w>0.5);
    return vec3f((screen.x-0.5)*scene.viewport.x/scene.viewport.y,0.5-screen.y,(foot-0.5)*2.0);
}
struct WorldCapture { @location(0) color:vec4f,@location(1) geometry:vec2f }
@fragment fn world_capture_fragment(i:Sprite)->WorldCapture {
    let c=textureSample(colors,nearest,i.uv);if c.a*i.optical.x<0.05 {discard;}
    // Keep the opaque layer behind glass; transmitting pixels cannot occlude their own source.
    if i.material.z*textureSample(surfaces,nearest,i.uv).b > .08 {discard;}
    var o:WorldCapture;
    let e=linear(textureSample(emission,nearest,i.uv).rgb)*i.material.w;
    o.color=vec4f(linear(c.rgb)*(scene.environment.x+0.35)+e,c.a*i.optical.x);
    o.geometry=vec2f(world_position(i.screen,i.world).z,i.world.z);return o;
}
fn world_illumination(screen:vec2f,pos:vec3f,n:vec3f,rough:f32,base:vec3f,metal:f32,reflectivity:f32)->vec3f {
    let tile=vec2u(screen*scene.viewport.xy/scene.world.w);
    let offset=(tile.y*u32(scene.world.z)+tile.x)*9u;
    var color=vec3f(0.0);
    for(var j=0u;j<min(light_tiles[offset],8u);j++) {
        let e=world_lights[light_tiles[offset+1u+j]];
        let p=vec3f((e.position.x-.5)*scene.viewport.x/scene.viewport.y,.5-e.position.y,e.position.z);
        let delta=p-pos;let radius=max(e.position.w,.025);let distance2=dot(delta,delta);
        let l=normalize(delta+vec3f(0.0,0.0,.001));let ndl=max(dot(n,l),0.0);
        let cutoff=1.0-smoothstep(radius,radius*2.0,sqrt(distance2));
        let attenuation=radius*radius/(distance2+radius*radius)*cutoff;
        let h=normalize(l+vec3f(0.0,0.0,1.0));
        let spec=pow(max(dot(n,h),0.0),mix(160.0,4.0,rough))*reflectivity;
        color+=(base*(.12+ndl*.88)*(1.0-metal*.5)+mix(vec3f(.12),base,metal)*spec)*e.color.rgb*attenuation*2.2;
    }
    return color;
}
fn world_reflection(origin:vec3f,direction:vec3f,rough:f32,exclude:f32)->vec3f {
    if scene.world.y<1.0 || scene.environment.z<.001 {return vec3f(0.0);}
    var previous=-1.0;var previous_id=-1.0;
    for(var step=0u;step<min(u32(scene.world.y),32u);step++) {
        let distance=.018+f32(step)*select(.025,.055,scene.world.x>1.5);
        let p=origin+direction*distance;
        let uv=vec2f(.5+p.x*scene.viewport.y/scene.viewport.x,.5-p.y);
        if any(uv<vec2f(0.0))||any(uv>=vec2f(1.0)){break;}
        let info=textureLoad(world_geometry,vec2i(uv*scene.viewport.xy),0).xy;
        if info.y<=0.0||abs(info.y-exclude)<.1 {previous_id=-1.0;continue;}
        let delta=info.x-p.z;
        if abs(delta)<.035 || (previous<0.0 && delta>=0.0 && previous_id==info.y) {
            let c=textureSampleLevel(world_color,nearest,uv,0.0).rgb;
            let blur=rough*rough*.008;
            let b=textureSampleLevel(world_color,nearest,clamp(uv+vec2f(blur,0.0),vec2f(0.0),vec2f(1.0)),0.0).rgb;
            return mix(c,b,.35)*(1.0-f32(step)/max(scene.world.y,1.0)*.5);
        }
        previous=delta;previous_id=info.y;
    }
    return vec3f(0.0);
}

// A fixed convex reflection probe for the composed mirror demonstration. It samples
// the animated opaque scene capture, so source movement/animation remains live.
// Unlike SSR this probe can cover a wide source field without a coincidental ray hit.
fn showcase_mirror(n:vec3f,rough:f32)->vec3f {
    let aspect=scene.viewport.x/scene.viewport.y;
    let uv=clamp(vec2f(.20,.38)+vec2f(-n.x/aspect,-n.y)*.42,vec2f(.001),vec2f(.999));
    let blur=rough*rough*.045;
    var color=vec3f(0.0);
    for(var j=0u;j<5u;j++) {
        let offsets=array<vec2f,5>(vec2f(0.0),vec2f(1.0,0.0),vec2f(-1.0,0.0),vec2f(0.0,1.0),vec2f(0.0,-1.0));
        let sample_uv=clamp(uv+offsets[j]*blur,vec2f(.001),vec2f(.999));
        let info=textureLoad(world_geometry,vec2i(sample_uv*scene.viewport.xy),0).xy;
        if abs(info.y-1.0)<.1 {color+=textureSampleLevel(world_color,nearest,sample_uv,0.0).rgb;}
    }
    return color*.2;
}
