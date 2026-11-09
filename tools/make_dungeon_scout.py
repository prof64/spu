#!/usr/bin/env python3
"""Rebuild the eight-direction Dungeon scout from its saved generated drawing.
Only palette/grid conversion, separation of existing pixels, and material authoring
are procedural. The character drawing comes from the retained image-generation source.
"""
from pathlib import Path
import sys, json, math
import numpy as np
from PIL import Image, ImageDraw
ROOT=Path(__file__).resolve().parents[1]
import make_examples as m
ART=ROOT/'web/art-source'; OUT=ROOT/'web/examples'
m.SOURCE=ART
NAMES=['South','SouthEast','East','NorthEast','North','NorthWest','West','SouthWest']
COLORS=['#1d1b23','#30242a','#44302f','#583b34','#6b4939','#835940','#a7734d',
 '#622d27','#81372c','#a44430','#bd5637','#d36a43','#e78352','#f49c65',
 '#c99669','#d9aa7b','#e9bf90','#f4d3a7','#ffe4bf','#fff0d4',
 '#1e2940','#2b3a59','#3c4e74','#51648e','#6b7fab','#8c9fc1',
 '#6e4b2d','#946230','#bb833d','#d69f4e','#edb963','#ffd378','#ffe89b','#fff6c4',
 '#eee4d8','#998481','#614e50']
PALETTE={'name':'Dungeon scout material ramps','colors':COLORS}

def lab(rgb):
    c=rgb.astype(float)/255
    lin=np.where(c>.04045,((c+.055)/1.055)**2.4,c/12.92)
    xyz=lin@np.array([[.4124,.2126,.0193],[.3576,.7152,.1192],[.1805,.0722,.9505]])/np.array([.95047,1,1.08883])
    f=np.where(xyz>.008856,np.cbrt(xyz),7.787*xyz+16/116)
    return np.stack([116*f[:,1]-16,500*(f[:,0]-f[:,1]),200*(f[:,1]-f[:,2])],axis=1)

def quantize(src,out,palette,width,max_colors):
    # CIE-Lab palette snap, majority vote per logical pixel, binary-alpha vote.
    # This is the deterministic native conversion used in the pixel-remake workflow.
    arr=np.array(Image.open(src).convert('RGBA'));h,w=arr.shape[:2]
    pal=np.array([[int(c[n:n+2],16) for n in (1,3,5)] for c in palette['colors']],dtype=float)
    packed=(arr[:,:,0].astype(np.uint32)<<16)|(arr[:,:,1].astype(np.uint32)<<8)|arr[:,:,2]
    unique,inverse=np.unique(packed,return_inverse=True)
    rgb=np.stack([(unique>>16)&255,(unique>>8)&255,unique&255],axis=1)
    mapping=np.linalg.norm(lab(rgb)[:,None,:]-lab(pal)[None,:,:],axis=2).argmin(axis=1)
    indices=mapping[inverse.reshape(h,w)]
    height=max(1,round(h*width/w));xs=np.linspace(0,w,width+1).round().astype(int);ys=np.linspace(0,h,height+1).round().astype(int)
    result=np.zeros((height,width,4),dtype=np.uint8)
    for y in range(height):
        for x in range(width):
            block=indices[ys[y]:ys[y+1],xs[x]:xs[x+1]];solid=arr[ys[y]:ys[y+1],xs[x]:xs[x+1],3]>=128
            if solid.mean()>=.5:
                chosen=np.bincount(block[solid],minlength=len(pal)).argmax();result[y,x,:3]=pal[chosen];result[y,x,3]=255
    Image.fromarray(result).save(out)

def build():
    ART.mkdir(exist_ok=True);OUT.mkdir(exist_ok=True)
    (ART/'dungeon-scout-palette.json').write_text(json.dumps(PALETTE,indent=2)+'\n')
    src=Image.open(ART/'dungeon-scout-generated.png').convert('RGBA')
    base=[]
    for i,name in enumerate(NAMES):
        x=i%4;y=i//4
        crop=src.crop((round(x*src.width/4),round(y*src.height/2),round((x+1)*src.width/4),round((y+1)*src.height/2)))
        a=crop.getchannel('A').point(lambda a:255 if a>=128 else 0);crop.putalpha(a);crop=crop.crop(crop.getbbox())
        crop.save(ART/f'dungeon-scout-{name}-crop.png')
        width=round(56*crop.width/crop.height)
        quantize(ART/f'dungeon-scout-{name}-crop.png',ART/f'dungeon-scout-{name}-native.png',PALETTE,width=width,max_colors=len(COLORS))
        im=Image.open(ART/f'dungeon-scout-{name}-native.png').convert('RGBA')
        frame=Image.new('RGBA',(72,72));frame.alpha_composite(im,((72-im.width)//2,64-im.height));base.append(frame)
    # Review the actual native drawings before material/rig segmentation.
    sheet=Image.new('RGBA',(72*4,86*2),(25,33,43,255));d=ImageDraw.Draw(sheet)
    for i,im in enumerate(base):
        sheet.alpha_composite(im,((i%4)*72,(i//4)*86));d.text(((i%4)*72+4,(i//4)*86+72),NAMES[i],fill=(195,213,219))
    sheet.resize((sheet.width*4,sheet.height*4),Image.Resampling.NEAREST).save(OUT/'dungeon-scout-directions.png')
    pack=m.Package('dungeon-scout','Dungeon scout',[
        m.mat('fur','Fur, cloth and leather',roughness=.9,normalStrength=.32,reflectivity=.25),
        m.mat('lantern','Amber lantern',roughness=.4,emission=3.3,normalStrength=.5,reflectivity=1.1),
    ],role='actor',native=72,anchor=[36,64],properties={'lightFuel':'amber moss','movement':'eight-direction'})
    pack.doc['provenance']={'source':'imagegen built-in, original character','sourceFile':'web/art-source/dungeon-scout-generated.png','conversionPalette':'web/art-source/dungeon-scout-palette.json'}
    for i,(name,im) in enumerate(zip(NAMES,base)):
        v=pack.view(name,72)
        arr=np.array(im);alpha=arr[:,:,3]>0;yy,xx=np.indices(alpha.shape)
        # The hand-held lamp occupies the outer lower quadrant. Select all existing
        # lamp pixels, retaining the hand with the body so its attachment stays planted.
        lamp_right=i<4
        lampzone=(xx>48 if lamp_right else xx<24)&(yy>45)&(yy<63)&alpha
        # Separate the two feet, hiding the cut behind the complete lower torso.
        footzone=(yy>=57)&(xx>=25)&(xx<=48)&alpha&~lampzone
        # North has the large hanging tail between the feet; preserve it in the body.
        if i==4:footzone &= (xx<29)|(xx>42)
        feet=[footzone&(xx<36),footzone&(xx>=36)]
        bodymask=alpha & ~lampzone & ~footzone
        regions=[('body',bodymask,'fur'),('foot-left',feet[0],'fur'),('foot-right',feet[1],'fur'),('lantern',lampzone,'lantern')]
        for part,mask,material in regions:
            layer=arr.copy();layer[:,:,3]=np.where(mask,255,0);layer[~mask,:3]=0;layer=Image.fromarray(layer)
            pack.part(v,part,parent='body' if part!='body' else None)
            # Normal relief follows actual native colour clusters, not a cylinder mask.
            lum=(arr[:,:,:3].astype(float)@np.array([.2126,.7152,.0722]))/255
            gy,gx=np.gradient(lum)
            normal=np.zeros_like(arr);normal[:,:,0]=np.clip(128-gx*48,100,156);normal[:,:,1]=np.clip(128+gy*48,100,156);normal[:,:,2]=251;normal[:,:,3]=np.where(mask,255,0)
            surface=np.zeros_like(arr);surface[:,:,:]=[255,255,255,210];surface[:,:,3]=np.where(mask,210,0)
            emission=np.zeros_like(arr)
            if part=='lantern':
                # Emit only the hot amber glass window, not the bronze housing.
                hot=mask&(arr[:,:,0]>185)&(arr[:,:,1]>112)&(arr[:,:,0]>arr[:,:,2]*1.18)
                emission[hot]=[255,147,36,255]
            pack.drawing(v,part,layer,(0,0),material,maps=[Image.fromarray(normal),Image.fromarray(surface),Image.fromarray(emission)])
        def tracks(walk=False):
            body=m.track('body','body')
            if walk:body['y']=[m.key(t,y,'step') for t,y in [(0,0),(7,-1),(14,0),(21,-1),(28,0)]]
            else:body['y']=[m.key(t,y,'step') for t,y in [(0,0),(50,0),(56,-1),(68,0),(100,0)]]
            result=[body]
            for n,part in enumerate(['foot-left','foot-right']):
                tr=m.track(part,part)
                if walk:
                    vals=[(0,0),(7,-2),(14,0),(21,1),(28,0)] if n==0 else [(0,0),(7,1),(14,0),(21,-2),(28,0)]
                    tr['y']=[m.key(t,y,'step') for t,y in vals]
                    tr['x']=[m.key(t,x,'step') for t,x in ([(0,0),(7,-1),(14,0),(21,1),(28,0)] if n==0 else [(0,0),(7,1),(14,0),(21,-1),(28,0)])]
                result.append(tr)
            tr=m.track('lantern','lantern',z=2)
            tr['emission']=[m.key(t,e) for t,e in ([(0,.9),(7,1.05),(14,.82),(21,1),(28,.9)] if walk else [(0,1),(25,.82),(55,1.12),(75,.9),(100,1)])]
            result.append(tr);return result
        v['animations']=[m.clip('Idle',100,tracks()),m.clip('Walk',28,tracks(True))]
    pack.save()
    frames=[]
    for t in range(0,28,7):
        row=Image.new('RGBA',(72*8,72),(25,33,43,255))
        for vi in range(8):row.alpha_composite(m.render(pack,vi,1,t),(vi*72,0))
        frames.append(row.resize((row.width*2,row.height*2),Image.Resampling.NEAREST).convert('RGB'))
    frames[0].save(OUT/'dungeon-scout-walk.gif',save_all=True,append_images=frames[1:],duration=120,loop=0)
    motion=Image.new('RGBA',(72*4,72*8),(25,33,43,255))
    for vi in range(8):
        for f,t in enumerate(range(0,28,7)):motion.alpha_composite(m.render(pack,vi,1,t),(f*72,vi*72))
    motion.resize((motion.width*2,motion.height*2),Image.Resampling.NEAREST).save(OUT/'dungeon-scout-walk.png')
    print('Wrote dungeon-scout.sprite; 8 views, Idle + Walk, 4 layers/view')
if __name__=='__main__':build()
