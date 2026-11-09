// Small 2.5D scene: ray/plane intersections choose the side of each animated card.
// +Z is toward the camera. A reflection can see an object's opposite drawing.
@group(2) @binding(0) var reflection_views:texture_2d<f32>;
fn captured(tile:f32,uv:vec2f)->vec4f {
    if any(uv<vec2f(0.0)) || any(uv>vec2f(1.0)) {return vec4f(0.0);}
    let q=clamp(uv,vec2f(0.5/256.0),vec2f(255.5/256.0));
    let c=textureSampleLevel(reflection_views,nearest,vec2f((tile+q.x)/4.0,q.y),0.0);
    // Capture uses source-over blending into transparent black (premultiplied storage).
    return vec4f(c.rgb/max(c.a,0.001),c.a);
}
fn trainer_marker(uv:vec2f,front:bool)->vec4f {
    let p=floor(uv*vec2f(12.0,24.0)); var c=vec3f(0.0); var a=0.0;
    if p.y>=18.0 && p.y<23.0 && ((p.x>=3.0&&p.x<5.0)||(p.x>=7.0&&p.x<9.0)) {c=vec3f(0.12,0.19,0.31);a=1.0;}
    if p.y>=9.0&&p.y<18.0&&p.x>=2.0&&p.x<10.0 {c=vec3f(0.60,0.18,0.72);a=1.0;}
    if p.y>=10.0&&p.y<16.0&&(p.x==1.0||p.x==10.0) {c=vec3f(0.71,0.45,0.26);a=1.0;}
    if p.y>=4.0&&p.y<9.0&&p.x>=3.0&&p.x<9.0 {c=select(vec3f(0.13,0.10,0.15),vec3f(0.82,0.57,0.35),front);a=1.0;}
    if p.y>=2.0&&p.y<5.0&&p.x>=2.0&&p.x<10.0 {c=vec3f(0.16,0.75,0.65);a=1.0;}
    if front&&p.y==6.0&&(p.x==4.0||p.x==7.0) {c=vec3f(0.015);}
    if front&&p.y>=10.0&&p.y<14.0&&p.x==6.0 {c=vec3f(0.95,0.82,0.30);}
    return vec4f(linear(c),a);
}
fn card_pixel(card:ReflectionCard,uv:vec2f,front:bool,rough:f32)->vec4f {
    if any(uv<vec2f(0.0))||any(uv>vec2f(1.0)){return vec4f(0.0);}
    if card.appearance.y<0.0 {return trainer_marker(uv,front);}
    let tile=select(card.appearance.z,card.appearance.y,front);
    var c=captured(tile,uv);
    if rough>0.13 {
        let d=rough*rough*0.045;
        // Filter in premultiplied space so transparent padding does not darken edges.
        var sum=vec4f(c.rgb*c.a,c.a)*2.0;
        let offsets=array<vec2f,4>(vec2f(d,0.0),vec2f(-d,0.0),vec2f(0.0,d),vec2f(0.0,-d));
        for(var k=0;k<4;k++){let s=captured(tile,uv+offsets[k]);sum+=vec4f(s.rgb*s.a,s.a);}
        c=vec4f(sum.rgb/max(sum.a,0.001),sum.a/6.0);
    }
    return c;
}
fn trace_cards(origin:vec3f,direction:vec3f,rough:f32,side:i32,exclude:i32)->vec4f {
    if abs(direction.z)<0.0001{return vec4f(0.0);}
    var closest=1e8; var result=vec4f(0.0);
    // Each shaded battler excludes only itself; either side can be the receiver.
    for(var k=0u;k<4u;k++) {
        if i32(k)==exclude {continue;}
        let card=scene.cards[k];
        if card.position.w<=0.0 {continue;}
        if side==1 && card.position.z<=0.0 {continue;}
        if side==(-1) && card.position.z>=0.0 {continue;}
        let t=(card.position.z-origin.z)/direction.z;
        if t<=0.001||t>=closest {continue;}
        let hit=origin+direction*t;
        let handedness=select(1.0,-1.0,direction.z>0.0);
        let uv=vec2f((hit.x-card.position.x)*handedness/(2.0*card.position.w)+0.5,0.5-(hit.y-card.position.y)/(2.0*card.appearance.x));
        let front=(-direction.z*card.appearance.w)>0.0;
        let pixel=card_pixel(card,uv,front,rough);
        if pixel.a>0.005 {closest=t;result=pixel;}
    }
    return result;
}
fn primary_objects(uv:vec2f,side:i32)->vec4f {
    if scene.camera.z<0.5 {return vec4f(0.0);}
    let p=vec2f((uv.x-0.5)*scene.viewport.x/scene.viewport.y,0.5-uv.y);
    let origin=vec3f(p+scene.camera.xy*3.0,3.0);
    return trace_cards(origin,normalize(vec3f(-scene.camera.xy,-1.0)),0.0,side,0);
}
fn storm_sky(direction:vec3f)->vec3f {
    let horizon=smoothstep(-0.25,0.8,direction.y);
    let cloud=0.78+sin(direction.x*11.0+direction.z*6.0+scene.viewport.z*0.045)*0.12;
    return mix(vec3f(0.025,0.065,0.085),vec3f(0.33,0.45,0.52),horizon)*cloud;
}
fn environment_reflection(origin:vec3f,direction:vec3f)->vec3f {
    if scene.environment.w>6.5 {
        let sky=mix(vec3f(0.016,0.024,0.04),vec3f(0.12,0.16,0.20),max(direction.y,0.0));
        return sky;
    }
    if scene.environment.w>5.5 {return storm_sky(direction);}
    var sky=mix(vec3f(0.004,0.008,0.015),vec3f(0.07,0.10,0.14),max(direction.y,0.0));
    if scene.environment.w>3.5 && scene.environment.w<4.5 {sky*=0.1;}
    // Test cards occupy an actual plane on one side of the sprite.
    let z=select(-0.8,0.8,scene.probe.z>0.5);
    if abs(direction.z)>0.001 {
        let t=(z-origin.z)/direction.z;
        if t>0.0 {
            let hit=origin+direction*t;
            let uv=vec2f(0.5+hit.x/(scene.viewport.x/scene.viewport.y),0.5-hit.y);
            let card=probe_at(uv);
            sky=mix(sky,card.rgb,card.a);
        }
    }
    return sky;
}
fn reflection_at(origin:vec3f,direction:vec3f,rough:f32,exclude:i32)->vec3f {
    let sky=environment_reflection(origin,direction);
    if scene.effects.w>0.5 || scene.environment.w>6.5 {
        let p=trace_cards(origin,direction,rough,0,exclude);
        return mix(sky,p.rgb,p.a);
    }
    return sky;
}
fn battle_center(card:ReflectionCard)->vec2f {
    let p=card.position.xy-scene.camera.xy*card.position.z;
    return vec2f(0.5+p.x*scene.viewport.y/scene.viewport.x,0.5-p.y);
}
fn battle_ground(uv:vec2f,base:vec3f)->vec3f {
    if scene.effects.w<0.5 {return base;}
    var c=base;
    let wet=scene.environment.w>5.5 && scene.environment.w<6.5;
    for(var k=0u;k<2u;k++) {
        let card=scene.cards[k]; let center=battle_center(card);
        let foot=center.y+card.position.w*0.42;
        let d=(uv-vec2f(center.x,foot))*vec2f(scene.viewport.x/scene.viewport.y,1.0);
        let radius=length(d/vec2f(card.position.w*0.64,card.position.w*0.105));
        let shadow=1.0-smoothstep(0.72,1.10,radius);
        c=mix(c,c*0.35,shadow*0.7);
        let rim=(1.0-smoothstep(0.025,0.055,abs(radius-1.05)))*0.17;
        c+=select(vec3f(0.12,0.19,0.15),vec3f(0.12,0.25,0.28),wet)*rim;
        if wet && uv.y>foot && scene.environment.z>0.0 {
            let ripple=sin(uv.y*170.0+scene.viewport.z*3.3)*0.004*scene.weather.x;
            let mapped=vec2f((uv.x-center.x)*scene.viewport.x/scene.viewport.y/(2.0*card.position.w)+0.5+ripple,
                0.5+(2.0*foot-uv.y-center.y)/(2.0*card.position.w));
            let s=captured(select(0.0,3.0,k==1u),mapped);
            let fade=clamp(1.0-(uv.y-foot)/0.25,0.0,1.0)*0.36*scene.weather.z*s.a;
            c=mix(c,s.rgb*vec3f(0.4,0.65,0.8),fade);
        }
    }
    return c;
}
fn rain_layer(uv:vec2f,layer:f32)->f32 {
    let aspect=scene.viewport.x/scene.viewport.y;
    let rate=scene.weather.x;
    let scale=38.0+layer*17.0;
    let speed=0.40+layer*0.23;
    let p=vec2f(uv.x*aspect-scene.weather.y*uv.y*0.25,uv.y)*vec2f(scale,scale*0.48)
        -vec2f(scene.weather.y*scene.viewport.z*speed*3.0,scene.viewport.z*speed*scale);
    let cell=floor(p); let f=fract(p); let seed=hash(cell+layer*19.0);
    if seed>clamp(rate*0.32,0.0,0.9) {return 0.0;}
    let width=0.012+layer*0.003;
    let streak=(1.0-smoothstep(width,width+0.018,abs(f.x-0.5)))*smoothstep(0.0,0.15,f.y)*(1.0-smoothstep(0.42,0.86,f.y));
    return streak*(0.20+layer*0.07);
}
fn rain_drops(uv:vec2f,front:bool)->vec4f {
    let layer=select(0.0,1.0,front);
    let a=rain_layer(uv,layer)+rain_layer(uv,layer+2.0);
    return vec4f(vec3f(0.34,0.52,0.62),clamp(a,0.0,0.65));
}
fn rainy_backdrop(uv:vec2f)->vec3f {
    let t=scene.viewport.z; let aspect=scene.viewport.x/scene.viewport.y;
    let cloud=sin(uv.x*8.0+t*0.025)*sin(uv.x*17.0-uv.y*5.0+t*0.018)*0.012;
    var c=mix(vec3f(0.027,0.052,0.071),vec3f(0.09,0.14,0.16),uv.y)+cloud;
    let mountain=0.43+sin(uv.x*11.0)*0.045+sin(uv.x*26.0)*0.023;
    if uv.y>mountain {c=vec3f(0.030,0.067,0.077);}
    let tree_x=floor(uv.x*42.0);let tree=abs(fract(uv.x*42.0)-0.5)*2.0;
    let treeline=0.55-hash(vec2f(tree_x,8.0))*0.10+tree*0.07;
    if uv.y>treeline {c=vec3f(0.016,0.042,0.047);}
    let horizon=select(0.70,0.52,scene.effects.w>0.5);
    if uv.y>horizon {
        let ground=(uv.y-horizon)/(1.0-horizon);
        c=mix(vec3f(0.035,0.064,0.072),vec3f(0.010,0.025,0.032),ground);
        let p=vec2f((uv.x-0.5)*aspect/(ground+0.25)*8.0,ground*14.0);
        let cell=floor(p);let local=fract(p)-0.5;let age=fract(t*(1.0+scene.weather.x*.3)+hash(cell)*9.0);
        let ring=(1.0-smoothstep(0.015,0.06,abs(length(local*vec2f(1.0,1.4))-age*0.55)))*(1.0-age);
        c+=vec3f(0.04,0.09,0.11)*ring*scene.weather.x*scene.weather.z;
        // A single-sprite preview uses its animated capture for the wet floor.
        if scene.weather.w<1.5 && scene.environment.z>0.0 {
            let mirror_y=1.48-uv.y;
            let ripple=sin(uv.y*170.0+t*3.3)*0.004*scene.weather.x;
            let mapped=vec2f((uv.x-0.5)*aspect+0.5+ripple,mirror_y);
            let tile=select(1.0,0.0,scene.camera.w>0.0);
            let s=captured(tile,mapped);
            let fade=clamp((1.0-ground)*0.48,0.0,0.48)*scene.weather.z*s.a;
            c=mix(c,s.rgb*vec3f(0.4,0.65,0.8),fade);
        }
        let wet_shine=pow(max(0.0,1.0-abs(uv.x-0.55)*1.8),5.0)*0.035*scene.weather.z;
        c+=vec3f(0.5,0.8,0.95)*wet_shine;
    }
    let rain=rain_drops(uv,false);return mix(battle_ground(uv,c),rain.rgb,rain.a);
}
