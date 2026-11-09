#!/usr/bin/env python3
"""Package the reviewed image-generated knight as editable material-separated pixel layers.
Source native.png is the fixed-palette majority-cell conversion of generated.png.
No image-generation service is invoked by this deterministic build.
"""
from pathlib import Path
import json, math, sys
from PIL import Image, ImageDraw
sys.path.insert(0,str(Path(__file__).resolve().parent))
from make_examples import Package, mat, canvas, key, track, clip, render, png
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'web/art-source/mirror-knight'
OUT=ROOT/'web/examples'
N=192; ANCHOR=(96,176)


def build():
    native=Image.open(SOURCE/'native.png').convert('RGBA')
    art=canvas(N);art.alpha_composite(native,(41,17))
    palette=json.loads((SOURCE/'palette.json').read_text())['colors']
    rgb=lambda c:tuple(int(c[i:i+2],16) for i in (1,3,5))
    steel={rgb(c) for c in palette[5:11]}; cyan={rgb(c) for c in palette[19:25]}
    region=Image.new('L',(N,N));d=ImageDraw.Draw(region)
    d.polygon([(41+x,17+y) for x,y in [(26,36),(42,43),(54,48),(53,86),(47,116),(25,143),(10,122),(0,91),(0,51),(14,41)]],fill=255)
    body=art.copy(); mirror=canvas(N); lenses=canvas(N)
    bm,bs,be=canvas(N),canvas(N),canvas(N)
    mm,ms,me=canvas(N),canvas(N),canvas(N)
    lm,ls,le=canvas(N),canvas(N),canvas(N)
    mask=Image.new('L',(N,N)); eye_mask=Image.new('L',(N,N))
    for y in range(N):
        for x in range(N):
            c=art.getpixel((x,y))
            if not c[3]:continue
            r,g,b=c[:3]; gold=r>b*1.13 and g>b*1.03
            is_steel=c[:3] in steel
            # Cloth folds remain rough dielectrics; brass and steel receive their own normal ramps.
            rough=190 if gold else 120 if is_steel else 255
            metal=230 if gold else 255 if is_steel else 0
            nx=math.sin((x-90)*.13)*(.16 if is_steel else .08)
            ny=math.cos((y-86)*.1)*(.15 if is_steel else .07)
            nz=math.sqrt(max(.01,1-nx*nx-ny*ny))
            bm.putpixel((x,y),(round((nx+.0)*127+128),round(ny*127+128),round(nz*127+128),255))
            bs.putpixel((x,y),(rough,metal,0,255));be.putpixel((x,y),(0,0,0,255))
            if region.getpixel((x,y)) and is_steel:
                # Broad convex shield: intentional lateral curvature, with a shallow vertical crown.
                dx=(x-68)/28;dy=(y-99)/60
                nx=max(-.82,min(.82,dx*.55-.12));ny=max(-.65,min(.65,-dy*.36))
                nz=math.sqrt(max(.05,1-nx*nx-ny*ny))
                mirror.putpixel((x,y),c);body.putpixel((x,y),(0,0,0,0));mask.putpixel((x,y),255)
                mm.putpixel((x,y),(round(nx*127+128),round(ny*127+128),round(nz*127+128),255));ms.putpixel((x,y),(255,255,0,255));me.putpixel((x,y),(0,0,0,255))
            elif c[:3] in cyan:
                lenses.putpixel((x,y),c);body.putpixel((x,y),(0,0,0,0));eye_mask.putpixel((x,y),255)
                lm.putpixel((x,y),(128,128,255,255));ls.putpixel((x,y),(170,100,0,255));le.putpixel((x,y),(12,min(255,g),min(255,b),255))
    p=Package('mirror-knight','Mirror knight',[
        mat('armor','Steel, brass and cloth',roughness=1,metalness=1,normalStrength=.55,reflectivity=.65),
        mat('mirror','Convex mirror shield',roughness=.065,metalness=1,normalStrength=1,reflectivity=1.5),
        mat('lenses','Living cyan lenses',roughness=.25,metalness=.1,emission=1.7,normalStrength=.1,reflectivity=.15),
    ],role='actor',native=N,anchor=list(ANCHOR),properties={'magnetism':.85,'guardArcDegrees':130,'shield':'convex silver'})
    p.doc['lighting']={'power':.08,'radius':.65}
    p.doc['provenance']='Original design generated with the built-in image_gen tool, converted to a fixed 28-color native pixel palette, and authored into material layers. Source and prompt under web/art-source/mirror-knight/.'
    v=p.view('Front',N)
    p.part(v,'root','Ground anchor')
    for id,title,img,matid,maps in [('armor','Armor and navy cloak',body,'armor',(bm,bs,be)),('mirror','Convex silver shield',mirror,'mirror',(mm,ms,me)),('lenses','Cyan visor and gems',lenses,'lenses',(lm,ls,le))]:
        p.part(v,id,title,'root');p.drawing(v,id,img,ANCHOR,matid,maps=maps)
    idle=[dict(part='root',x=[key(0,96)],y=[key(0,176)],scaleY=[key(0,1),key(70,.992),key(140,1)]),track('armor','armor'),track('mirror','mirror',z=1),track('lenses','lenses',z=2,emission=[key(0,.7),key(60,1.3),key(100,.8),key(140,.7)])]
    guard=[dict(part='root',x=[key(0,96),key(18,92,'easeOut'),key(60,92),key(96,96)],y=[key(0,176)],scaleY=[key(0,1),key(18,.965),key(60,.965),key(96,1)],rotation=[key(0,0),key(18,-1.5),key(60,-1.5),key(96,0)]),track('armor','armor'),track('mirror','mirror',z=1),track('lenses','lenses',z=2,emission=[key(0,.7),key(18,3),key(48,2),key(60,2),key(96,.7)])]
    v['animations']=[clip('Idle',140,idle),clip('Guard',96,guard,False)]
    p.doc['conversion']={'nativeArtwork':[109,159],'canvas':[192,192],'palette':'web/art-source/mirror-knight/palette.json','source':'web/art-source/mirror-knight/generated.png','method':'CIE Lab fixed-palette majority cell conversion; binary alpha; no dithering'}
    p.save()
    for id,im in [('native-canvas',art),('shield-mask',mask),('emission-mask',eye_mask),('body',body),('shield',mirror),('lenses',lenses)]:im.save(SOURCE/(id+'.png'))
    # Transparent sheet records actual guard poses and preserves nearest-neighbor review.
    sheet=Image.new('RGBA',(N*4,N),(20,26,36,255))
    for i,t in enumerate([0,18,48,96]):sheet.alpha_composite(render(p,0,1,t),(N*i,0))
    sheet.resize((N*4*2,N*2),Image.Resampling.NEAREST).save(OUT/'mirror-knight-motion.png')
    preview=[]
    for t in range(0,97,4):
        f=Image.new('RGBA',(N,N),(20,26,36,255));f.alpha_composite(render(p,0,1,t));preview.append(f.resize((N*3,N*3),Image.Resampling.NEAREST).convert('RGB'))
    preview[0].save(OUT/'mirror-knight-preview.gif',save_all=True,append_images=preview[1:],duration=67,loop=0)
    print(f'Mirror knight: {len(p.entries)} resources, {len(v["parts"])} parts, {len(v["animations"])} clips, {(OUT/"mirror-knight.sprite").stat().st_size:,} bytes')
    return p

if __name__=='__main__':build()
