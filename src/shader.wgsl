// Experimental 2.5D model: animated two-sided cards at finite depths, one reflection bounce.
// Linear-light HDR compositing, then one display conversion in display.wgsl.
struct Emitter { position:vec4f,color:vec4f }
struct ReflectionCard {position:vec4f,appearance:vec4f}
struct Scene { viewport:vec4f,light:vec4f,environment:vec4f,probe:vec4f,effects:vec4f,emitters:array<Emitter,16>,weather:vec4f,camera:vec4f,cards:array<ReflectionCard,4>,world:vec4f }
@group(0) @binding(0) var<uniform> scene:Scene;
@group(1) @binding(0) var colors:texture_2d<f32>;
@group(1) @binding(1) var normals:texture_2d<f32>;
@group(1) @binding(2) var surfaces:texture_2d<f32>;
@group(1) @binding(3) var emission:texture_2d<f32>;
@group(1) @binding(4) var nearest:sampler;

fn linear(c:vec3f)->vec3f { return pow(max(c,vec3f(0.0)),vec3f(2.2)); }
struct Screen { @builtin(position) position:vec4f,@location(0) uv:vec2f }
@vertex fn fullscreen(@builtin(vertex_index) i:u32)->Screen{
    let p=array<vec2f,3>(vec2f(-1.0,-1.0),vec2f(3.0,-1.0),vec2f(-1.0,3.0));
    var o:Screen;o.position=vec4f(p[i],0.0,1.0);o.uv=vec2f(p[i].x*0.5+0.5,0.5-p[i].y*0.5);return o;
}
fn probe_at(uv:vec2f)->vec4f{
    if scene.probe.w<0.5{return vec4f(0.0);}
    let aspect=scene.viewport.x/scene.viewport.y;
    let d=(uv-scene.probe.xy)*vec2f(aspect,1.0);
    if scene.environment.w>4.5 && scene.environment.w<5.5 {
        // Two HDR studio cards. Their visible position and reflected position share one function.
        if abs(d.y)<0.30 {
            if abs(d.x+0.38)<0.10 {return vec4f(vec3f(0.06,1.7,3.6)*5.0,1.0);}
            if abs(d.x-0.38)<0.10 {return vec4f(vec3f(4.0,0.40,0.06)*5.0,1.0);}
        }
        if abs(d.y+0.34)<0.012 && abs(d.x)<0.46 {return vec4f(vec3f(3.0),1.0);}
        return vec4f(0.0);
    }
    // A striped amber wafer and a cyan disk give transmission and reflection spatial references.
    if abs(d.x)<0.045 && abs(d.y)<0.31 {
        let stripe=select(0.42,1.0,fract((d.y+d.x)*19.0)>0.5);
        return vec4f(vec3f(0.70,0.28,0.045)*stripe,1.0);
    }
    let circle=d-vec2f(-0.16,0.10);
    if length(circle)<0.075{
        let s=sqrt(max(0.0,1.0-dot(circle,circle)/(0.075*0.075)));
        return vec4f(vec3f(0.025,0.34,0.43)*(0.45+s*0.6),1.0);
    }
    return vec4f(0.0);
}
fn hash(p:vec2f)->f32 { return fract(sin(dot(p,vec2f(127.1,311.7)))*43758.5453); }
fn cave(uv:vec2f)->vec3f {
    let aspect=scene.viewport.x/scene.viewport.y;
    // Angular strata, an arch, a distant recess, and a receding stone floor.
    let floor_y=uv.y+select(0.0,0.20,scene.effects.w>0.5);
    let floor_mask=step(0.73+0.025*sin(uv.x*18.0),floor_y);
    let arch=abs(uv.x-0.51)+max(0.0,0.38-uv.y)*0.85;
    let wall=step(0.325+0.017*sin(uv.y*22.0),arch);
    let rim=step(0.267+0.014*sin(uv.y*17.0),arch);
    let rock_mask=max(max(wall,floor_mask),rim*0.55);
    let floor_uv=vec2f((uv.x-0.5)/(max(floor_y-0.60,0.08))*1.6,1.4/max(floor_y-0.55,0.08));
    let warped=uv*vec2f(7.0*aspect,6.0)+vec2f(sin(uv.y*13.0+uv.x*6.0)*0.65,sin(uv.x*15.0+uv.y*4.0)*0.5);
    let rock_uv=mix(warped,floor_uv+vec2f(sin(floor_uv.y*1.7)*0.35,sin(floor_uv.x*2.0)*0.15),floor_mask);
    let row=floor(rock_uv.y);
    let stagger=vec2f(hash(vec2f(row,2.0))*0.65,0.0);
    let cell=floor(rock_uv+stagger);
    let f=fract(rock_uv+stagger);
    let shade=0.60+hash(cell)*0.40;
    let facet=select(0.64,1.0,f.x+f.y>0.8+hash(cell+3.0)*0.3);
    let seam=smoothstep(0.003,0.02,min(min(f.x,1.0-f.x),min(f.y,1.0-f.y)));
    var albedo=vec3f(0.19,0.24,0.29)*shade*facet*mix(0.40,1.0,seam);
    albedo*=mix(0.07,1.0,rock_mask);
    // Jagged ceiling needles and a few near silhouettes ground the procedural cave.
    let column=floor(uv.x*15.0);
    let tooth=(0.5-abs(fract(uv.x*15.0)-0.5))*2.0;
    let tip=(0.06+hash(vec2f(column,4.0))*0.20)*tooth;
    let ceiling=step(uv.y,tip);
    albedo=mix(albedo,vec3f(0.13,0.18,0.23)*mix(0.55,1.0,step(0.5,fract(uv.x*15.0))),ceiling);
    let facet_normal=normalize(vec3f((f.x-0.5)*0.65,(f.y-0.5)*0.45,1.0));
    var illumination=vec3f(0.015,0.027,0.052)+vec3f(scene.environment.x*0.28);
    let lamp=vec3f((scene.light.x-uv.x)*aspect,uv.y-scene.light.y,max(scene.light.z,0.05));
    illumination+=vec3f(0.6,0.72,0.85)*scene.light.w*max(dot(normalize(lamp),facet_normal),0.0)/(1.0+dot(lamp,lamp)*4.0);
    for(var j=0u;j<min(u32(scene.effects.z),16u);j++) {
        let e=scene.emitters[j];
        let delta=vec3f((e.position.x-uv.x)*aspect,uv.y-e.position.y,e.position.z);
        let radius=max(e.position.w,0.045);
        let attenuation=radius*radius/(dot(delta,delta)+radius*radius);
        illumination+=e.color.rgb*attenuation*attenuation*scene.effects.y*8.0;
    }
    var c=albedo*illumination;
    // Cool air in the recess keeps the silhouette readable with the lights off.
    c+=vec3f(0.0006,0.0018,0.004)*(1.0-wall)*(1.0-floor_mask);
    let vignette=1.0-smoothstep(0.3,0.95,length((uv-0.5)*vec2f(aspect,1.0)))*0.55;
    return c*vignette;
}
fn backdrop(uv:vec2f)->vec3f{
    if scene.environment.w< -0.5 {
        let cell=vec2u(floor(uv*scene.viewport.xy/16.0));
        let checker=f32((cell.x+cell.y)%2u);
        return linear(vec3f(mix(0.76,0.80,checker)));
    }
    if scene.world.x>0.5 {return vec3f(.012,.019,.025);}
    if scene.environment.w>5.5 && scene.environment.w<6.5 {return rainy_backdrop(uv);}
    if scene.environment.w>6.5 {
        let radius=length((uv-vec2f(0.5,0.45))*vec2f(scene.viewport.x/scene.viewport.y,1.0));
        var c=mix(vec3f(0.025,0.044,0.055),vec3f(0.004,0.010,0.017),smoothstep(0.0,0.85,radius));
        let grid=fract(uv*scene.viewport.xy/48.0);c+=vec3f(0.004)*select(0.0,1.0,min(grid.x,grid.y)<0.02);
        let p=primary_objects(uv,-1);return battle_ground(uv,mix(c,p.rgb,p.a));
    }
    let aspect=scene.viewport.x/scene.viewport.y;
    let radius=length((uv-vec2f(0.5,0.45))*vec2f(aspect,1.0));
    var c=mix(vec3f(0.014,0.022,0.032),vec3f(0.0035,0.006,0.010),smoothstep(0.1,0.9,radius));
    if scene.environment.w<0.5{
        let cell=fract(uv*scene.viewport.xy/32.0);
        let line=select(0.0,1.0,min(cell.x,cell.y)<0.025);
        c+=vec3f(0.003,0.004,0.005)*line;
    }else if scene.environment.w<1.5{
        let checker=(u32(floor(uv.x*scene.viewport.x/32.0))+u32(floor(uv.y*scene.viewport.y/32.0)))%2u;
        c=mix(vec3f(0.045),vec3f(0.09),f32(checker));
    }else if scene.environment.w<2.5{
        let stripe=step(0.5,fract((uv.x+uv.y*0.25)*20.0));
        c=mix(vec3f(0.025,0.13,0.14),vec3f(0.32,0.08,0.035),stripe);
    }else if scene.environment.w<3.5{ c=vec3f(0.0007,0.001,0.002); }
    else if scene.environment.w<4.5{ c=cave(uv); }
    else {
        c=vec3f(0.0035,0.005,0.009);
        let floor_grid=fract(vec2f((uv.x-0.5)/max(uv.y-0.56,0.08)*3.0,1.0/max(uv.y-0.56,0.08)));
        if uv.y>0.73 { c+=vec3f(0.006)*select(0.2,1.0,min(floor_grid.x,floor_grid.y)<0.035); }
    }
    if scene.probe.z<0.5{let p=probe_at(uv);c=mix(c,p.rgb,p.a);}
    return battle_ground(uv,c);
}
@fragment fn background(i:Screen)->@location(0)vec4f{return vec4f(backdrop(i.uv),1.0);}
@fragment fn foreground(i:Screen)->@location(0)vec4f{
    if scene.environment.w< -0.5 {return vec4f(0.0);}
    if scene.world.x>0.5 {if scene.environment.w>5.5 && scene.environment.w<6.5{return rain_drops(i.uv,true);}return vec4f(0.0);}
    if scene.environment.w>6.5 {return primary_objects(i.uv,1);}
    if scene.environment.w>5.5 {return rain_drops(i.uv,true);}
    if scene.probe.z<0.5{return vec4f(0.0);}
    let p=probe_at(i.uv);return p;
}
struct PartInput {
    @location(0) basis:vec4f,@location(1) placement:vec4f,@location(2) uv:vec4f,
    @location(3) material:vec4f,@location(4) optical:vec4f,@location(5) normal_matrix:vec4f,@location(6) world:vec4f
}
struct Sprite {
    @builtin(position)position:vec4f,
    @location(0)uv:vec2f,@location(1)screen:vec2f,
    @location(2)@interpolate(flat)material:vec4f,@location(3)@interpolate(flat)optical:vec4f,
    @location(4)@interpolate(flat)normal_matrix:vec4f,
    @location(5)@interpolate(flat)reflectivity:f32,
    @location(6)@interpolate(flat)object:f32,
    @location(7)@interpolate(flat)world:vec4f
}
@vertex fn sprite_vertex(i:PartInput,@builtin(vertex_index) vertex:u32)->Sprite{
    let corners=array<vec2f,6>(vec2f(0.0,0.0),vec2f(1.0,0.0),vec2f(0.0,1.0),vec2f(0.0,1.0),vec2f(1.0,0.0),vec2f(1.0,1.0));
    let p=corners[vertex];let pixel=i.placement.xy+i.basis.xy*p.x+i.basis.zw*p.y;
    var o:Sprite;o.position=vec4f(pixel.x/scene.viewport.x*2.0-1.0,1.0-pixel.y/scene.viewport.y*2.0,0.0,1.0);
    o.screen=pixel/scene.viewport.xy;o.uv=i.uv.xy+i.uv.zw*p;o.material=i.material;o.optical=i.optical;o.normal_matrix=i.normal_matrix;o.reflectivity=i.placement.z;o.object=i.placement.w;o.world=i.world;return o;
}
@vertex fn capture_vertex(i:PartInput,@builtin(vertex_index) vertex:u32)->Sprite {
    let corners=array<vec2f,6>(vec2f(0.0,0.0),vec2f(1.0,0.0),vec2f(0.0,1.0),vec2f(0.0,1.0),vec2f(1.0,0.0),vec2f(1.0,1.0));
    let p=corners[vertex];let pixel=i.placement.xy+i.basis.xy*p.x+i.basis.zw*p.y;
    var o:Sprite;o.position=vec4f(pixel.x/1024.0*2.0-1.0,1.0-pixel.y/256.0*2.0,0.0,1.0);
    o.uv=i.uv.xy+i.uv.zw*p;o.material=i.material;o.optical=i.optical;return o;
}
@fragment fn capture_fragment(i:Sprite)->@location(0)vec4f {
    let c=textureSample(colors,nearest,i.uv);if c.a<0.005{discard;}
    let e=linear(textureSample(emission,nearest,i.uv).rgb)*i.material.w;
    return vec4f(linear(c.rgb)+e,c.a*i.optical.x);
}
@fragment fn sprite_fragment(i:Sprite)->@location(0)vec4f{
    let tex=textureSample(colors,nearest,i.uv);
    if tex.a<0.005{discard;}
    let alpha=tex.a*i.optical.x;
    if scene.viewport.w>0.5 && scene.viewport.w<1.5{return vec4f(linear(tex.rgb),alpha);}
    let normal_tex=textureSample(normals,nearest,i.uv).xyz*2.0-1.0;
    // Map +Y-up tangent normals through +Y-down image transforms, then back to light coordinates.
    let local=vec2f(normal_tex.x,-normal_tex.y)*i.optical.w;
    let nm=i.normal_matrix;
    let transformed=vec2f(nm.x*local.x+nm.z*local.y,nm.y*local.x+nm.w*local.y);
    var n=normalize(vec3f(transformed.x,-transformed.y,max(normal_tex.z,0.02)));
    if scene.world.x>0.5 && i.world.w>0.5 {n=normalize(vec3f(n.x,2.0*n.z+n.y,n.z-2.0*n.y));}
    let surface=textureSample(surfaces,nearest,i.uv);
    let emit=linear(textureSample(emission,nearest,i.uv).rgb)*i.material.w;
    if scene.viewport.w>1.5 && scene.viewport.w<2.5{return vec4f(linear(n*0.5+0.5),alpha);}
    if scene.viewport.w>2.5 && scene.viewport.w<3.5{return vec4f(linear(surface.rgb),alpha);}
    if scene.viewport.w>3.5 && scene.viewport.w<4.5{return vec4f(emit,alpha);}
    let base=linear(tex.rgb);
    var roughness=clamp(i.material.x*surface.r,0.045,1.0);
    if scene.environment.w>5.5 && scene.environment.w<6.5 {
        roughness=mix(roughness,min(roughness,0.16),scene.weather.z*0.75);
        let streak=sin(i.screen.x*scene.viewport.x*.12+scene.viewport.z*.8)*sin(i.screen.y*scene.viewport.y*.06-scene.viewport.z*2.0);
        n=normalize(n+vec3f(streak*0.07,streak*0.02,0.0)*scene.weather.z*scene.weather.x);
    }
    let metal=clamp(i.material.y*surface.g,0.0,1.0);
    let transmission=clamp(i.material.z*surface.b,0.0,1.0);
    let aspect=scene.viewport.x/scene.viewport.y;
    let light_delta=vec3f((scene.light.x-i.screen.x)*aspect,i.screen.y-scene.light.y,max(scene.light.z,0.02));
    let l=normalize(light_delta);let v=normalize(vec3f(select(scene.camera.xy,vec2f(0.0),scene.world.x>0.5),1.0));let h=normalize(l+v);
    let ndl=max(dot(n,l),0.0);let ndv=max(dot(n,v),0.001);let ndh=max(dot(n,h),0.0);
    let a2=pow(roughness,4.0);let d=a2/(3.14159265*pow(ndh*ndh*(a2-1.0)+1.0,2.0));
    let k=pow(roughness+1.0,2.0)/8.0;
    let g=(ndl/(ndl*(1.0-k)+k))*(ndv/(ndv*(1.0-k)+k));
    let dielectric=pow((i.optical.z-1.0)/(i.optical.z+1.0),2.0);
    let f0=mix(vec3f(dielectric),base,metal);
    let f=f0+(vec3f(1.0)-f0)*pow(1.0-max(dot(h,v),0.0),5.0);
    let direct_strength=scene.light.w/(0.7+dot(light_delta,light_delta)*0.8);
    let ambient=select(scene.environment.x,max(scene.environment.x,.55),scene.world.x>1.5 && i.world.w>0.5);
    var lit=base*(ambient+ndl*direct_strength*0.6)*(1.0-metal*0.65);
    lit+=min(vec3f(6.0),d*g*f/max(4.0*ndl*ndv,0.001))*ndl*direct_strength*scene.environment.z*i.reflectivity;
    let reflected=reflect(-v,n);
    let object=select(0,i32(i.object)-1,i.object>0.5);
    let depth=select(0.0,scene.cards[u32(object)].position.z,scene.effects.w>0.5);
    var world=vec3f(vec2f((i.screen.x-0.5)*aspect,0.5-i.screen.y)+scene.camera.xy*depth,depth);
    var environment_color=vec3f(0.0);
    if scene.world.x>0.5 {
        world=world_position(i.screen,i.world);
        if roughness<0.85 && i.reflectivity>0.001 {
            environment_color=world_reflection(world,reflected,roughness,i.world.z);
            if scene.world.x>2.5 && scene.world.x<3.5 && scene.world.y>0.5 && abs(i.world.z-2.0)<.1 && metal>.5 {
                environment_color=showcase_mirror(n,roughness);
            }
        }
        lit+=world_illumination(i.screen,world,n,roughness,base,metal,scene.environment.z*i.reflectivity);
    }else {environment_color=reflection_at(world,reflected,roughness,object);}
    if scene.viewport.w>4.5 {return vec4f(environment_color,alpha);}
    let softbox=pow(max(0.0,dot(reflected,normalize(vec3f(-0.45,0.65,1.0)))),mix(65.0,2.0,roughness));
    lit+=(environment_color*1.8+vec3f(softbox*select(0.8,0.0,scene.environment.w>5.5)))*f*scene.environment.z*i.reflectivity*(1.0-roughness*0.65);
    let bend=vec2f(-n.x/aspect,n.y)*(i.optical.z-1.0)*i.optical.y*surface.a*select(0.035,0.11,scene.world.x>3.5 && scene.world.x<4.5);
    var behind=backdrop(clamp(i.screen+bend,vec2f(0.0),vec2f(1.0)));
    if scene.world.x>0.5 {
        let uv=clamp(i.screen+bend,vec2f(0.0),vec2f(.9999));
        let info=textureLoad(world_geometry,vec2i(uv*scene.viewport.xy),0).xy;
        if abs(info.y-i.world.z)>.1 {behind=textureSampleLevel(world_color,nearest,uv,0.0).rgb;}
    }else if scene.effects.w>0.5 {
        let through=trace_cards(world,-v,roughness,0,object);
        behind=mix(behind,through.rgb,through.a);
    }
    behind*=mix(vec3f(1.0),base,0.25);
    lit=mix(lit,behind,transmission*(1.0-min(f.x,0.95)))+emit;
    return vec4f(lit,alpha);
}
