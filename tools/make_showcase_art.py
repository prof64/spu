#!/usr/bin/env python3
"""Convert the saved generated artwork to editable native-pixel material packages.

Pillow only; no network/generation on rebuild. Original prompts and high-resolution
alpha sources are retained in web/art-source. No old example is overwritten.
"""
from pathlib import Path
from PIL import Image, ImageDraw
import colorsys, io, json, math
import make_examples as base

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'web/art-source'
OUT=ROOT/'web/examples'
base.SOURCE=SOURCE
base.LICENSE='Original artwork generated with OpenAI image_gen for Sprite Lab, then converted to native pixels and authored material/animation data. Dedicated under CC0 1.0 to the extent copyright applies. No extracted game artwork.'
SIZE=192
FIRE=['#14111e','#211b2b','#302436','#403144','#514053','#635363','#756675','#8d7c88','#a494a0',
      '#272637','#35354b','#464960','#5a6078','#727a90','#9294a1',
      '#321323','#4b1726','#70202b','#8d292d','#b0392d','#d64521','#ee5818','#ff751b','#ff9c24','#ffc132','#ffdd53','#fff28a','#fff7bc',
      '#402933','#65433f','#8a6355','#b48b6d','#d0ad88','#eace9d','#fff0bd']
ICE=['#062e49','#06455c','#075c71','#10768a','#1a8ca1','#2da4b7','#4abaca','#73d1db','#a3e7eb','#d1f5f1','#f4fffa',
     '#123d68','#235d8d','#367bb0','#579cd3','#80b9ed','#abd7fb',
     '#242555','#393273','#514593','#6b5baf','#8577cf','#a093e5','#beb5f5','#ded4ff',
     '#4c2d75','#744a9b','#9b6cc5','#c48fea','#dfb8fa','#04b9ce','#30e6eb','#a1ffef']

def native(name,palette):
    src=Image.open(SOURCE/f'{name}-generated.png').convert('RGBA')
    src=src.crop(src.getbbox())
    span=160 if name=='prismcoil' else 176
    scale=span/max(src.size);w,h=round(src.width*scale),round(src.height*scale)
    colors=[tuple(bytes.fromhex(c[1:])) for c in palette]
    pal=Image.new('P',(1,1));pal.putpalette([v for c in colors for v in c]+list(colors[0])*(256-len(colors)))
    indexed=src.convert('RGB').quantize(palette=pal,dither=Image.Dither.NONE)
    alpha=src.getchannel('A')
    im=Image.new('RGBA',(SIZE,SIZE));ox=(SIZE-w)//2;oy=178-h
    # Each native cell votes for an explicit material-ramp color, with majority alpha.
    for y in range(h):
        for x in range(w):
            box=(round(x*src.width/w),round(y*src.height/h),round((x+1)*src.width/w),round((y+1)*src.height/h))
            aa=list(alpha.crop(box).getdata());cc=list(indexed.crop(box).getdata())
            votes={}
            for a,c in zip(aa,cc):
                if a>=128:votes[c]=votes.get(c,0)+1
            if sum(votes.values())*2<len(aa):continue
            c=max(votes,key=lambda k:(votes[k],-k))
            im.putpixel((ox+x,oy+y),colors[c if c<len(colors) else 0]+(255,))
    im.save(SOURCE/f'{name}-native.png')
    (SOURCE/f'{name}-palette.json').write_text(json.dumps({'name':name,'colors':palette},indent=2)+'\n')
    # Editable native rows: exact symbol/palette round trip, including transparency.
    symbols='0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'
    lookup={c:symbols[i] for i,c in enumerate(colors)}
    rows=[''.join(lookup.get(im.getpixel((x,y))[:3],'.') if im.getpixel((x,y))[3] else '.' for x in range(SIZE)) for y in range(SIZE)]
    (SOURCE/f'{name}-pixels.json').write_text(json.dumps({'width':SIZE,'height':SIZE,'transparent':'.','palette':dict(zip(symbols,palette)),'rows':rows},indent=2)+'\n')
    return im

def maps(im,ice=False):
    normal=Image.new('RGBA',im.size);surface=Image.new('RGBA',im.size);emit=Image.new('RGBA',im.size)
    for y in range(im.height):
        for x in range(im.width):
            r,g,b,a=im.getpixel((x,y))
            if not a:continue
            hue,sat,value=colorsys.rgb_to_hsv(r/255,g/255,b/255)
            if ice:
                # Facet directions follow the actual teal / violet / light color planes.
                nx=-.55 if hue<.57 else .48 if hue>.64 else .12
                ny=.42 if value>.78 else -.32 if value<.42 else .06
                rough=.60 if value>.82 else 1.0
                transmission=.85 if value<.82 else .22
                thickness=.55 if value>.8 else .95
                if g>160 and b>160 and r<g*.5 and 22<x<110 and y<133:
                    emit.putpixel((x,y),(int(r*.35),int(g*.7),int(b*.75),255))
            else:
                nx=(.34 if b>r else -.25)*(1 if value>.3 else -1)
                ny=.42 if value>.36 else -.22
                rough=1 if r>g*1.4 else .70
                transmission=0;thickness=.9
                # Orange/gold magma is luminous; pale claws and violet armor are not.
                if r>160 and r>b*1.8 and g>b*1.15 and (y<163 or b<70):
                    emit.putpixel((x,y),(r,g,min(b,80),255))
            nz=math.sqrt(1-nx*nx-ny*ny)
            normal.putpixel((x,y),(round((nx*.5+.5)*255),round((ny*.5+.5)*255),round((nz*.5+.5)*255),255))
            surface.putpixel((x,y),(round(rough*255),255,round(transmission*255),round(thickness*255)))
    return normal,surface,emit

def track(part,image,x=0,y=0,z=0,**kw):return base.track(part,image,x,y,z,**kw)
key=base.key

def review_frame(p,vi,t):
    """Unlit exported pose review with the delivered part opacity and transform keys."""
    v=p.doc['views'][vi];tracks=v['animations'][1]['tracks'];out=Image.new('RGBA',(SIZE,SIZE))
    images={d['id']:d for d in v['images']}
    for tr in sorted(tracks,key=lambda tr:base.sample(tr.get('z',[]),t,0)):
        d=images[base.sample(tr['image'],t,None)]
        im=Image.open(io.BytesIO(p.entries[d['color']])).convert('RGBA')
        opacity=base.sample(tr.get('opacity',[]),t,1)
        im.putalpha(im.getchannel('A').point(lambda a:round(a*opacity)))
        sx=base.sample(tr.get('scaleX',[]),t,1);sy=base.sample(tr.get('scaleY',[]),t,1)
        r=math.radians(base.sample(tr.get('rotation',[]),t,0));cs=math.cos(r);sn=math.sin(r)
        a,b,c,e=cs*sx,sn*sx,-sn*sy,cs*sy
        x=base.sample(tr.get('x',[]),t,0)+a*d['x']+c*d['y']
        y=base.sample(tr.get('y',[]),t,0)+b*d['x']+e*d['y'];det=a*e-b*c
        layer=im.transform(out.size,Image.Transform.AFFINE,(e/det,-c/det,(c*y-e*x)/det,-b/det,a/det,(b*x-a*y)/det),Image.Resampling.NEAREST)
        out.alpha_composite(layer)
    return out

def effects(p,v,ice):
    im=Image.new('RGBA',(9,13));d=ImageDraw.Draw(im)
    if ice:
        d.polygon([(4,0),(8,6),(4,12),(0,6)],fill='#4a83bd')
        d.polygon([(4,0),(4,12),(0,6)],fill='#85f5ff')
        d.polygon([(4,0),(8,6),(4,6)],fill='#d2faff')
    else:
        d.polygon([(4,1),(3,5),(6,4),(7,8),(4,12),(1,8)],fill='#ff8521')
        d.polygon([(4,5),(5,9),(3,10),(2,8)],fill='#fff28a')
    normals=Image.new('RGBA',im.size,(150,155,247,255))
    surfaces=Image.new('RGBA',im.size,(128,255,0,128))
    p.drawing(v,'mote',im,(4,6),'mote',maps=(normals,surfaces,im))
    for i in range(8):p.part(v,f'mote-{i}')

def package(kind,ice=False):
    ident=f'showcase-{kind}';name='Prismcoil' if ice else 'Cindermaw'
    mats=[base.mat('body','Faceted glass' if ice else 'Obsidian and magma',roughness=.16 if ice else .58,metalness=.12 if ice else .16,transmission=.68 if ice else 0,reflectivity=2.2 if ice else 1.2,normalStrength=1,ior=1.52,thickness=1.8 if ice else 1,emission=1.2 if ice else 2.7),base.mat('mote','Crystal mote' if ice else 'Living ember',roughness=.12,emission=3.5 if ice else 5,reflectivity=1.5)]
    p=base.Package(ident,name,mats,role='actor',native=SIZE,anchor=[96,178],properties={'heat':0 if ice else 1800,'material':'glass' if ice else 'volcanic rock'},style='Original generated pixel-art master, native palette conversion, authored surface maps')
    p.doc['provenance']=base.LICENSE
    p.doc['lighting']={'power':.15 if ice else 5.0,'radius':.8 if ice else 1.5}
    for facing in ['Front'] if ice else ['Front','Back']:
        source=kind+('-back' if facing=='Back' else '')
        im=native(source,ICE if ice else FIRE);v=p.view(facing,SIZE)
        p.part(v,'body');p.drawing(v,'body',im,(96,178),'body',maps=maps(im,ice));effects(p,v,ice)
        for action,length in [('Idle',180),('Surge',180)]:
            active=action=='Surge'
            body=track('body','body',96,178,0)
            body['scaleY']=[key(0,1),key(90,1.015 if not active else .975),key(108,1.035 if active else 1.014),key(180,1)]
            body['y']=[key(0,178),key(80,177 if not active else 178),key(108,169 if active and ice else 175 if active else 177),key(180,178)]
            body['emission']=[key(0,.8),key(72,.55 if active else 1.05),key(104,3.4 if active else .8),key(132,2.3 if active else .9),key(180,.8)]
            tracks=[body]
            for i in range(8):
                phase=i/8*math.tau
                tr=track(f'mote-{i}','mote',0,0,1)
                tr['x']=[];tr['y']=[];tr['opacity']=[];tr['rotation']=[]
                tr['scaleX']=[key(0,.55 if not active else .8)];tr['scaleY']=tr['scaleX']
                for t in range(0,181,15):
                    q=t/180
                    if ice:
                        angle=phase+q*math.tau
                        r=65+(20*math.sin(q*math.pi)**8 if active else 0)
                        x=96+math.cos(angle)*r;y=99+math.sin(angle)*53
                        opacity=(.35+.65*math.sin(q*math.pi)**4) if active else .45
                    else:
                        life=(q+i/8)%1
                        x=54+i*11+math.sin(phase+life*5)*8;y=116-life*(92 if active else 62)
                        opacity=math.sin(life*math.pi)*(.95 if active else .45)
                    tr['x'].append(key(t,x));tr['y'].append(key(t,y));tr['opacity'].append(key(t,opacity));tr['rotation'].append(key(t,q*160+i*31))
                tracks.append(tr)
            v['animations'].append(base.clip(action,length,tracks))
    p.save()
    # Save the drawn body as the thumbnail, avoiding incidental orbiting motes.
    Image.open(SOURCE/f'{kind}-native.png').save(OUT/f'{ident}.png')
    for vi,v in enumerate(p.doc['views']):
        # Four native cels at 2× in one row, with no interpolated display animation.
        sheet=Image.new('RGBA',(SIZE*8,SIZE*2),(17,26,34,255))
        for i,t in enumerate([0,65,105,145]):sheet.alpha_composite(review_frame(p,vi,t).resize((SIZE*2,SIZE*2),Image.Resampling.NEAREST),(i*SIZE*2,0))
        sheet.save(OUT/f'{ident}-{v["id"]}-motion.png')
        frames=[]
        for t in range(0,180,3):
            frame=Image.new('RGBA',(SIZE,SIZE),(17,26,34,255));frame.alpha_composite(review_frame(p,vi,t));frames.append(frame.resize((SIZE*3,SIZE*3),Image.Resampling.NEAREST).convert('RGB'))
        frames[0].save(OUT/f'{ident}-{v["id"]}-preview.gif',save_all=True,append_images=frames[1:],duration=50,loop=0)
    return p

if __name__=='__main__':
    packs=[package('cindermaw'),package('prismcoil',True)]
    sheet=Image.new('RGBA',(1152,480),(17,26,34,255))
    for i,n in enumerate(['cindermaw','cindermaw-back','prismcoil']):
        im=Image.open(SOURCE/f'{n}-native.png');sheet.alpha_composite(im.resize((384,384),Image.Resampling.NEAREST),(i*384,32));ImageDraw.Draw(sheet).text((i*384+16,450),n,fill='#b6d6df')
    sheet.save(OUT/'showcase-heroes.png')
    print('\n'.join(f'{p.id}: {len(p.doc["views"])} views; {(OUT/(p.id+".sprite")).stat().st_size:,} bytes' for p in packs))
