#!/usr/bin/env python3
"""Original native-pixel examples for Sprite Lab. Requires Pillow; no network.

All artwork is drawn directly on its native grid. Exported packages retain separate
parts, cels, maps, pivots, clocks and custom game metadata. Run from any directory.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import copy, io, json, math, zipfile

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'web/examples'
SOURCE=ROOT/'tools/example_art'
OUT.mkdir(parents=True,exist_ok=True)
LICENSE='Original artwork created for Sprite Lab. Dedicated to the public domain under CC0 1.0; https://creativecommons.org/publicdomain/zero/1.0/ . No game artwork is included.'

def rgba(c):
    if isinstance(c,str):
        c=c.lstrip('#');return tuple(int(c[i:i+2],16) for i in (0,2,4))+(255,)
    return c

def canvas(w,h=None): return Image.new('RGBA',(w,h or w))
def poly(im,pts,color,outline=None): ImageDraw.Draw(im).polygon(pts,fill=rgba(color),outline=rgba(outline) if outline else None)
def line(im,pts,color,width=1): ImageDraw.Draw(im).line(pts,fill=rgba(color),width=width)
def rect(im,box,color): ImageDraw.Draw(im).rectangle(box,fill=rgba(color))
def ellipse(im,box,color,outline=None): ImageDraw.Draw(im).ellipse(box,fill=rgba(color),outline=rgba(outline) if outline else None)
def png(im):
    b=io.BytesIO();im.save(b,format='PNG',optimize=True);return b.getvalue()
def key(t,v,e='easeInOut'):return dict(tick=t,value=v,ease=e)
def track(part,image,x=0,y=0,z=0,**kwargs):
    return dict(part=part,image=[key(0,image,'step')],x=[key(0,x)],y=[key(0,y)],z=[key(0,z)],**kwargs)
def clip(name,length,tracks,loop=True):return dict(id=name.lower().replace(' ','-'),name=name,length=length,looping=loop,tracks=tracks)
def material(id,name,**kw):
    return dict(id=id,name=name,roughness=.7,metalness=0,transmission=0,emission=0,normalStrength=.65,reflectivity=1,**kw)
# Avoid duplicate kwargs when overriding defaults.
def mat(id,name,**kw):
    d=material(id,name);d.update(kw);return d

class Package:
    def __init__(self,id,name,materials,role='prop',native=32,**extra):
        self.id=id;self.entries={};self.doc=dict(format='sprite-lab',version=0,name=name,ticksPerSecond=60,required=['core-0'],materials=materials,views=[],role=role,nativeSize=[native,native],worldSize=1,provenance=LICENSE,**extra)
        self.rest=[]
    def view(self,name,size):
        v=dict(id=name.lower(),name=name,canvas=dict(x=0,y=0,width=size,height=size),parts=[],images=[],animations=[])
        self.doc['views'].append(v);return v
    def drawing(self,v,id,image,pivot,material,emissive=None,faceted=False,maps=None):
        bbox=image.getbbox() or (0,0,1,1)
        image=image.crop(bbox)
        w,h=image.size
        if maps:
            normal,surface,emission=[m.crop(bbox) for m in maps]
        else:
            normal=canvas(w,h);surface=canvas(w,h);emission=canvas(w,h)
            for y in range(h):
                for x in range(w):
                    r,g,b,a=image.getpixel((x,y))
                    if not a:continue
                    nx=(x+.5-w/2)/max(w/2,1)*.48
                    ny=-(y+.5-h/2)/max(h/2,1)*.48
                    if faceted:nx=round(nx*3)/3;ny=round(ny*3)/3
                    nz=math.sqrt(max(.1,1-nx*nx-ny*ny))
                    normal.putpixel((x,y),(int((nx*.5+.5)*255),int((ny*.5+.5)*255),int((nz*.5+.5)*255),255))
                    surface.putpixel((x,y),(255,255,255,210))
                    if emissive:
                        e=emissive((r,g,b,a),x+bbox[0],y+bbox[1])
                        emission.putpixel((x,y),tuple(e)+(255,))
        prefix=f'{v["id"]}/{id}'
        d=dict(id=id,name=id.replace('-',' ').title(),x=bbox[0]-pivot[0],y=bbox[1]-pivot[1],material=material)
        for suffix,img in [('color',image),('normal',normal),('surface',surface),('emission',emission)]:
            path=f'{prefix}-{suffix}.png';self.entries[path]=png(img);d[suffix]=path
        v['images'].append(d);return id
    def part(self,v,id,name=None,parent=None):
        p=dict(id=id,name=name or id.replace('-',' ').title())
        if parent:p['parent']=parent
        v['parts'].append(p)
    def save(self):
        data=dict(self.entries)
        data['sprite.json']=(json.dumps(self.doc,indent=2,ensure_ascii=False)+'\n').encode()
        data['LICENSE.txt']=LICENSE.encode()
        with zipfile.ZipFile(OUT/f'{self.id}.sprite','w') as z:
            for name,value in sorted(data.items()):
                info=zipfile.ZipInfo(name,(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED if name.endswith('.json') else zipfile.ZIP_STORED;z.writestr(info,value)
        # A plain editable manifest beside the drawing source records the native contract.
        (SOURCE/f'{self.id}.json').write_text(json.dumps(self.doc,indent=2,ensure_ascii=False)+'\n')
        rest=render(self,0,0,0)
        rest.save(OUT/f'{self.id}.png')
        return self

def sample(keys,t,default):
    if not keys:return default
    prev=keys[0]
    for n in keys[1:]:
        if t<n['tick']:
            f=(t-prev['tick'])/(n['tick']-prev['tick']);e=prev.get('ease','step')
            f=max(0,min(1,f))
            f=0 if e=='step' else f*f*(3-2*f) if e=='easeInOut' else f
            if isinstance(prev['value'],(int,float)) and not isinstance(prev['value'],bool):return prev['value']+(n['value']-prev['value'])*f
            return prev['value']
        prev=n
    return prev['value']

def render(pack,vi=0,ci=0,t=0):
    v=pack.doc['views'][vi];c=v['canvas'];im=canvas(int(c['width']),int(c['height']));an=v['animations'][ci]
    t=t%an['length'] if an['looping'] else min(t,an['length']);tracks={x['part']:x for x in an['tracks']};poses={};images={d['id']:d for d in v['images']}
    def pose(part):
        id=part['id']
        if id in poses:return poses[id]
        tr=tracks.get(id,{});x=sample(tr.get('x',[]),t,0);y=sample(tr.get('y',[]),t,0);r=sample(tr.get('rotation',[]),t,0);sx=sample(tr.get('scaleX',[]),t,1);sy=sample(tr.get('scaleY',[]),t,1)
        cs=math.cos(math.radians(r));sn=math.sin(math.radians(r));m=[cs*sx,sn*sx,-sn*sy,cs*sy,x,y]
        if part.get('parent'):
            pm=pose(next(p for p in v['parts'] if p['id']==part['parent']))[0];a,b,c,d,px,py=pm;aa,bb,cc,dd,xx,yy=m;m=[a*aa+c*bb,b*aa+d*bb,a*cc+c*dd,b*cc+d*dd,a*xx+c*yy+px,b*xx+d*yy+py]
        result=(m,tr);poses[id]=result;return result
    for part in sorted(v['parts'],key=lambda p:sample(tracks.get(p['id'],{}).get('z',[]),t,0)):
        m,tr=pose(part);img=sample(tr.get('image',[]),t,None)
        if not img or not sample(tr.get('visible',[]),t,True):continue
        d=images[img];src=Image.open(io.BytesIO(pack.entries[d['color']])).convert('RGBA');a,b,c,dd,x,y=m;x+=a*d['x']+c*d['y'];y+=b*d['x']+dd*d['y'];x-=v['canvas']['x'];y-=v['canvas']['y'];det=a*dd-b*c
        coeff=(dd/det,-c/det,(c*y-dd*x)/det,-b/det,a/det,(b*x-a*y)/det)
        layer=src.transform(im.size,Image.Transform.AFFINE,coeff,Image.Resampling.NEAREST)
        im.alpha_composite(layer)
    return im

# ---- A complete, two-view articulated original battle creature. ----
INK='#1a1d32';DARK='#33314f';BLUE='#4c5575';MID='#697a9a';PALE='#b9d6cf';CREAM='#f7e7b0';RED='#843c50';RUST='#bb4c48';ORANGE='#e87849';GOLD='#ffbf63';YELLOW='#ffe5a1'

def phoenix_wing(left=True,back=False):
    im=canvas(96)
    pts=[(42,41),(31,30),(20,24),(9,28),(18,37),(5,35),(12,44),(23,49),(10,48),(19,58),(32,56),(27,64),(39,59),(47,49)]
    poly(im,pts,RED,INK)
    poly(im,[(39,41),(29,34),(18,29),(24,39),(13,39),(25,47),(19,51),(33,50),(30,57),(42,51)],RUST)
    poly(im,[(38,40),(29,35),(23,33),(31,42),(21,43),(33,49),(30,53),(39,49),(43,44)],ORANGE)
    line(im,[(17,29),(26,34),(34,42)],GOLD)
    line(im,[(12,39),(22,43),(30,48)],ORANGE)
    line(im,[(18,51),(27,54),(33,52)],GOLD)
    poly(im,[(38,39),(33,42),(36,48),(42,49),(46,45),(44,41)],BLUE,INK)
    line(im,[(39,40),(36,43),(40,46)],PALE)
    if back:
        poly(im,[(40,39),(32,37),(34,45),(43,50),(47,45)],DARK,INK)
        line(im,[(36,40),(42,45)],MID)
    if not left:im=im.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    return im

def emberwing():
    p=Package('emberwing','Emberwing',[mat('plumage','Feather',roughness=.74),mat('ember','Ember feather',roughness=.5,emission=2.5),mat('beak','Horn',roughness=.3)],role='actor',native=96,anchor=[48,88],properties={'heat':.8,'flight':True},style='Original Gen 5-inspired articulated battle sprite')
    for back in [False,True]:
        v=p.view('Back' if back else 'Front',96)
        images={};pivots={'tail':(48,61),'foot-l':(41,68),'foot-r':(56,68),'wing-l':(43,43),'wing-r':(54,43),'body':(48,54),'head':(48,33)}
        # Tail is a complete feather fan behind the body, retaining hidden roots.
        tail=canvas(96);poly(tail,[(44,56),(52,56),(58,67),(64,81),(56,77),(54,88),(48,83),(42,89),(42,77),(34,81),(39,66)],RED,INK)
        poly(tail,[(46,61),(50,61),(54,75),(52,82),(48,78),(44,82),(44,72)],ORANGE)
        poly(tail,[(47,65),(49,65),(50,76),(47,80),(45,77)],GOLD)
        images['tail']=tail
        for id,x in [('foot-l',41),('foot-r',56)]:
            im=canvas(96);poly(im,[(x-3,63),(x+3,63),(x+3,70),(x+6,74),(x+5,76),(x-5,76),(x-7,74),(x-2,70)],RUST,INK);poly(im,[(x-1,67),(x+1,67),(x+1,72),(x+4,74),(x-4,74)],GOLD);line(im,[(x-3,74),(x-4,76)],CREAM);images[id]=im
        images['wing-l']=phoenix_wing(True,back);images['wing-r']=phoenix_wing(False,back)
        body=canvas(96)
        poly(body,[(40,35),(51,34),(59,40),(61,52),(59,64),(53,71),(42,70),(35,63),(34,51),(36,41)],DARK,INK)
        poly(body,[(40,38),(50,36),(55,41),(56,52),(53,65),(45,68),(39,62),(37,51)],BLUE)
        if not back:
            poly(body,[(43,39),(49,38),(54,44),(53,58),(49,66),(43,63),(40,53)],PALE)
            poly(body,[(44,40),(48,40),(51,45),(49,58),(46,62),(43,55)],CREAM)
            for y in [47,53,59]:line(body,[(41,y),(46,y+2),(51,y-1)],GOLD)
        else:
            poly(body,[(42,37),(49,36),(54,40),(51,48),(53,55),(49,62),(50,69),(44,65),(41,58),(43,50),(39,44)],BLUE)
            for y in [43,50,57,64]:poly(body,[(44,y-3),(49,y-4),(52,y),(47,y+2),(43,y)],MID)
            line(body,[(42,39),(39,46),(39,56)],PALE)
        images['body']=body
        head=canvas(96)
        poly(head,[(38,20),(39,10),(45,17),(49,14),(57,15),(63,21),(63,31),(58,39),(47,40),(39,35),(35,28)],DARK,INK)
        poly(head,[(39,20),(40,14),(45,20),(50,18),(56,18),(60,22),(58,31),(51,35),(42,32),(38,27)],BLUE)
        poly(head,[(40,20),(43,22),(48,19),(55,19),(58,22),(54,25),(47,25),(41,23)],MID)
        poly(head,[(48,15),(52,8),(56,10),(54,17)],RUST,INK)
        poly(head,[(53,17),(59,10),(62,11),(58,20)],ORANGE,INK)
        line(head,[(53,12),(53,15)],GOLD)
        if not back:
            poly(head,[(36,26),(43,23),(49,25),(46,31),(39,33),(35,30)],PALE)
            poly(head,[(38,26),(43,25),(46,26),(44,30),(39,30)],CREAM)
            rect(head,(40,26,42,28),INK);rect(head,(40,26,40,26),'#ffffff')
            poly(head,[(34,29),(39,29),(40,32),(35,36),(35,33),(30,32)],GOLD,INK)
            line(head,[(33,31),(37,31)],YELLOW)
            poly(head,[(49,27),(56,24),(58,27),(54,31),(49,30)],DARK)
            rect(head,(52,26,54,28),GOLD);rect(head,(52,26,52,27),INK)
            line(head,[(38,34),(44,36),(49,35)],MID)
        else:
            poly(head,[(40,21),(45,20),(50,22),(49,29),(54,34),(49,38),(42,34),(39,28)],MID)
            poly(head,[(46,22),(50,22),(52,27),(50,32),(46,33),(43,28)],BLUE)
            poly(head,[(59,25),(64,26),(66,29),(62,30),(60,29)],GOLD,INK)
            rect(head,(59,24,60,25),CREAM);rect(head,(60,24,60,25),INK)
            line(head,[(40,24),(43,27),(43,32)],PALE)
        images['head']=head
        openhead=head.copy()
        if not back:
            poly(openhead,[(33,31),(39,31),(39,35),(35,38),(34,35),(31,34)],INK)
            poly(openhead,[(34,35),(38,34),(36,38)],ORANGE)
        else:poly(openhead,[(62,28),(66,29),(63,33),(61,31)],RUST,INK)
        glow=lambda c,x,y: (230,91,25) if c[:3]==rgba(GOLD)[:3] and y>42 else (60,10,6) if c[:3]==rgba(ORANGE)[:3] and y>43 else (0,0,0)
        for i,id in enumerate(['tail','wing-r','foot-r','foot-l','body','wing-l','head']):
            pivot=pivots[id];parent=None if id=='body' else 'body';p.part(v,id,parent=parent)
            p.drawing(v,id,images[id],pivot,'ember' if id in ['tail','wing-l','wing-r'] else 'plumage',emissive=glow)
        p.drawing(v,'head-call',openhead,pivots['head'],'plumage')
        base=[]
        for z,id in enumerate(['tail','wing-r','foot-r','foot-l','body','wing-l','head']):
            px,py=pivots[id];parentpivot=(0,0) if id=='body' else pivots['body'];base.append(track(id,id,px-parentpivot[0],py-parentpivot[1],z))
        idle=copy.deepcopy(base)
        for tr in idle:
            id=tr['part']
            if id=='body':tr['y']=[key(0,54),key(45,53),key(90,54),key(135,55),key(180,54)]
            if id.startswith('wing'):sign=1 if id=='wing-l' else -1;tr['rotation']=[key(0,-sign*2),key(90,sign*3),key(180,-sign*2)]
            if id=='tail':tr['rotation']=[key(0,-4),key(60,3),key(120,-2),key(180,-4)];tr['emission']=[key(0,.5),key(70,1),key(130,.7),key(180,.5)]
        flare=copy.deepcopy(base)
        for tr in flare:
            id=tr['part']
            if id.startswith('wing'):
                sg=1 if id=='wing-l' else -1;tr['rotation']=[key(0,0),key(18,-sg*8),key(30,sg*24),key(48,sg*18),key(78,0),key(120,0)]
                tr['emission']=[key(0,.6),key(28,.6),key(36,3.5),key(64,1.7),key(94,.6)]
            if id=='body':tr['y']=[key(0,54),key(18,57),key(30,51),key(48,52),key(78,54)]
            if id=='head':tr['image']=[key(0,'head','step'),key(28,'head-call','step'),key(60,'head','step')]
            if id=='tail':tr['emission']=[key(0,.6),key(30,3),key(60,2),key(94,.6)]
        v['animations']=[clip('Idle',180,idle),clip('Flare',120,flare)]
    return p.save()

# ---- Eight hand-composed dungeon directions, with four native walk cels. ----
SCOUT={'ink':'#23302e','dark':'#59453d','fur':'#a16c48','light':'#ce9661','cream':'#edc997','green':'#366457','mint':'#6b9c79','pale':'#a8c99a','boot':'#39433d','gold':'#eebf65','eye':'#162b32'}

def scout_frame(direction,phase):
    # East-facing drawings are mirrored only for west; north/south/diagonals have distinct anatomy.
    west=direction in ['SouthWest','West','NorthWest'];base={'SouthWest':'SouthEast','West':'East','NorthWest':'NorthEast'}.get(direction,direction)
    back=base in ['North','NorthEast'];side=base=='East';diag=base in ['SouthEast','NorthEast'];bob=0 if phase in [0,2] else -1
    parts={id:canvas(40) for id in ['tail','body','head','lantern']};c=SCOUT
    t=parts['tail'];poly(t,[(15,24),(10,21),(7,22),(7,27),(11,30),(17,29),(20,26)],c['dark'],c['ink']);poly(t,[(12,23),(9,23),(9,26),(13,28),(17,27)],c['light']);poly(t,[(8,23),(8,26),(10,28),(12,28),(12,25)],c['cream'])
    body=parts['body'];dy=phase%2;leg1=1 if phase==1 else -1 if phase==3 else 0;leg2=-leg1
    for x,yy in [(16,leg1),(23,leg2)]:
        rect(body,(x-2,27+yy,x+2,31+yy),c['ink']);rect(body,(x-1,27+yy,x+1,29+yy),c['dark']);rect(body,(x-2,30+yy,x+2,31+yy),c['boot'])
    poly(body,[(14,19),(24,18),(27,24),(25,29),(14,29),(12,25)],c['green'],c['ink'])
    poly(body,[(15,20),(23,20),(24,26),(22,28),(15,27)],c['mint']);line(body,[(15,20),(14,24),(16,26)],c['pale'])
    if back:
        poly(body,[(16,20),(23,20),(25,22),(24,27),(16,27),(15,24)],c['dark'],c['ink']);rect(body,(17,21,23,25),c['fur']);line(body,[(17,22),(23,22)],c['cream']);rect(body,(20,23,21,24),c['gold'])
    else:
        line(body,[(17,20),(20,23),(23,20)],c['cream']);rect(body,(19,24,21,26),c['dark']);rect(body,(20,24,20,24),c['gold'])
    # Arms attach under the neck; a right hand meets the lantern handle.
    if side or diag:poly(body,[(24,21),(27,22),(27,25),(25,26),(23,24)],c['light'],c['ink'])
    else:
        poly(body,[(12,21),(15,22),(15,26),(12,26),(11,24)],c['fur'],c['ink']);poly(body,[(25,21),(28,23),(27,26),(24,25)],c['light'],c['ink'])
    h=parts['head'];yb=bob
    def hp(pts,col,ol=None):poly(h,[(x,y+yb) for x,y in pts],col,ol)
    def hr(box,col):rect(h,(box[0],box[1]+yb,box[2],box[3]+yb),col)
    hp([(11,10),(10,5),(14,6),(17,10),(22,8),(26,4),(28,5),(28,14),(29,18),(26,22),(17,23),(11,19),(10,15)],c['fur'],c['ink'])
    hp([(12,9),(12,7),(15,10)],c['dark']);hp([(24,10),(27,6),(26,12)],c['dark'])
    hp([(13,11),(18,10),(23,11),(27,14),(26,18),(22,20),(16,19),(12,16)],c['light'])
    if back:
        hp([(13,12),(18,10),(23,12),(25,17),(21,21),(15,19)],c['fur'])
        hp([(17,10),(21,10),(23,13),(20,15),(18,14)],c['light'])
        if diag:
            hp([(26,14),(29,16),(27,18),(25,18)],c['cream']);hr((26,13,27,14),c['eye'])
        line(h,[(14,19+yb),(19,22+yb),(25,20+yb)],c['dark'])
    elif side:
        hp([(22,13),(28,14),(31,17),(29,20),(23,20),(20,17)],c['cream']);hr((24,13,25,15),c['eye']);hr((24,13,24,13),'#ffffff');hr((29,16,31,17),c['ink'])
        hp([(12,11),(17,10),(18,16),(15,19),(12,16)],c['fur'])
    elif diag:
        hp([(18,15),(24,14),(28,16),(28,20),(23,22),(18,20)],c['cream']);hr((21,13,22,15),c['eye']);hr((21,13,21,13),'#ffffff');hr((27,15,28,17),c['eye']);hr((26,19,28,20),c['ink'])
        hp([(12,12),(16,12),(17,16),(14,18),(11,16)],c['fur'])
    else:
        hp([(12,15),(16,15),(19,18),(23,15),(27,15),(27,20),(22,23),(17,22),(12,19)],c['cream']);hr((14,13,15,15),c['eye']);hr((24,13,25,15),c['eye']);hr((14,13,14,13),'#ffffff');hr((24,13,24,13),'#ffffff');hr((19,18,21,19),c['ink']);hr((20,20,20,21),c['dark'])
    # A scarf secures the backpack, and remains a separate silhouette under the head.
    line(h,[(14,21+yb),(20,23+yb),(25,21+yb)],c['green'],2)
    l=parts['lantern'];lx=29 if side or diag else 28;ly=25+leg2//2
    line(l,[(lx-1,ly),(lx-1,ly-3),(lx+1,ly-3),(lx+1,ly)],c['dark'])
    rect(l,(lx-2,ly,lx+2,ly+5),c['ink']);rect(l,(lx-1,ly+1,lx+1,ly+3),c['gold']);rect(l,(lx,ly+1,lx,ly+2),'#fff1b8');line(l,[(lx-2,ly+5),(lx+2,ly+5)],c['light'])
    if back and not diag:parts['lantern']=canvas(40) # Far hand is occluded in a true back view.
    if west:parts={id:im.transpose(Image.Transpose.FLIP_LEFT_RIGHT) for id,im in parts.items()}
    return parts

def moss_scout():
    p=Package('moss-scout','Moss Scout',[mat('cloth','Cloth and fur',roughness=.9,reflectivity=.35),mat('light','Lantern glass',roughness=.25,emission=2.6,transmission=.18)],role='actor',native=40,anchor=[20,32],properties={'walkSpeed':2.5,'collisionRadius':.22},style='Original PMD-inspired eight-direction dungeon character')
    directions=['South','SouthEast','East','NorthEast','North','NorthWest','West','SouthWest']
    for name in directions:
        v=p.view(name,40);v['direction']=name;v['anchor']=[20,31]
        for id in ['tail','body','head','lantern']:p.part(v,id,parent='body' if id!='body' else None)
        for phase in range(4):
            for id,im in scout_frame(name,phase).items():
                glow=lambda c,x,y:(250,165,50) if c[0]>220 and c[1]>150 else (0,0,0)
                p.drawing(v,f'{id}-{phase}',im,(20,24),'light' if id=='lantern' else 'cloth',emissive=glow if id=='lantern' else None)
        idle=[];walk=[]
        for z,id in enumerate(['tail','body','head','lantern']):
            x,y=(20,24) if id=='body' else (0,0)
            ti=track(id,id+'-0',x,y,z);tw=track(id,id+'-0',x,y,z)
            tw['image']=[key(t,f'{id}-{frame}','step') for t,frame in [(0,0),(7,1),(14,2),(21,3),(28,0)]]
            if id=='head':ti['y']=[key(0,0,'step'),key(45,-1,'step'),key(90,0,'step'),key(135,0,'step')]
            if id=='lantern':ti['emission']=[key(0,.75),key(30,1),key(60,.8),key(90,1.1),key(135,.75)]
            idle.append(ti);walk.append(tw)
        v['animations']=[clip('Idle',135,idle),clip('Walk',28,walk)]
    return p.save()

# ---- Generic game objects. ----
def lantern():
    p=Package('lantern','Lantern',[mat('brass','Brass',roughness=.28,metalness=.85,properties={'magnetism':.25}),mat('glass','Warm glass',roughness=.12,transmission=.35,emission=3.0,ior=1.5)],native=48,anchor=[24,42],properties={'fuelSeconds':240,'pickup':True})
    v=p.view('Front',48)
    frame=canvas(48);glass=canvas(48);flame=canvas(48)
    line(frame,[(17,15),(16,11),(17,7),(20,5),(27,5),(30,8),(30,12),(28,15)],INK,3);line(frame,[(18,12),(18,8),(21,7),(26,7),(28,9),(28,12)],'#cba56a')
    poly(frame,[(16,14),(29,14),(33,18),(31,20),(15,20),(12,18)],'#674b39',INK);line(frame,[(16,15),(29,15),(31,17),(14,17)],'#d7b577')
    poly(frame,[(15,34),(31,34),(34,38),(32,41),(14,41),(12,38)],'#76563c',INK);rect(frame,(16,36,30,38),'#c59a58');line(frame,[(15,39),(31,39)],'#e2c181')
    poly(glass,[(17,19),(29,19),(31,23),(30,33),(27,35),(18,35),(15,31),(15,24)],'#ba8143',INK)
    poly(glass,[(18,21),(27,21),(29,24),(28,31),(26,33),(18,32),(17,28)],'#e7bb70');rect(glass,(19,23,25,31),'#f5d698');line(glass,[(18,22),(17,25),(17,29)],'#fff0c1');line(glass,[(28,25),(28,30)],'#f8dfaa')
    for x in [15,29]:rect(frame,(x,20,x+1,35),'#735239');line(frame,[(15,21),(15,33)],'#d3a95e')
    poly(flame,[(22,22),(25,25),(26,29),(24,32),(21,31),(20,28),(22,27)],'#ffeec0');poly(flame,[(23,26),(24,28),(24,30),(22,30)],'#ffffff')
    for id,img,piv,matid,z in [('glass',glass,(24,24),'glass',0),('flame',flame,(24,24),'glass',1),('frame',frame,(24,24),'brass',2)]:
        p.part(v,id,parent='frame' if id!='frame' else None)
        glow=lambda c,x,y:(255,180,60) if c[0]>175 and 20<y<35 else (0,0,0)
        p.drawing(v,id,img,piv,matid,emissive=glow if id!='frame' else None)
    tracks=[track('glass','glass',0,0,0),track('flame','flame',0,0,1),track('frame','frame',24,24,2)]
    for tr in tracks:
        if tr['part']!='frame':tr['emission']=[key(t,x) for t,x in [(0,.7),(13,1.1),(26,.8),(43,1),(60,.7)]]
        if tr['part']=='flame':tr['scaleX']=[key(0,1),key(13,.78),key(26,1.15),key(43,.9),key(60,1)]
    v['animations']=[clip('Flicker',60,tracks)];return p.save()

def vial():
    p=Package('glass-vial','Glass vial',[mat('glass','Blue glass',roughness=.08,transmission=.72,ior=1.47,thickness=.6,reflectivity=1.5),mat('liquid','Luminous liquid',roughness=.2,transmission=.35,emission=.65),mat('cork','Cork',roughness=.94,reflectivity=.15)],native=48,anchor=[24,42],properties={'liquid':'mana','volume':.75,'fragile':True})
    v=p.view('Front',48);glass=canvas(48);liquid=canvas(48);cork=canvas(48)
    poly(glass,[(19,10),(29,10),(29,17),(35,23),(37,29),(36,36),(32,41),(16,41),(12,36),(11,29),(13,23),(19,17)],'#386985',INK)
    poly(glass,[(20,13),(27,13),(27,19),(33,24),(35,29),(34,35),(31,38),(17,38),(14,34),(14,27),(17,22),(21,18)],'#6ca8b6')
    line(glass,[(20,16),(20,19),(15,25),(14,30),(15,34)],'#d4eece',2);line(glass,[(18,38),(29,39),(32,37)],'#95d2cc')
    poly(liquid,[(15,27),(20,25),(27,27),(33,26),(34,31),(32,36),(29,38),(18,38),(15,35),(14,31)],'#257a8d')
    poly(liquid,[(15,28),(21,27),(28,29),(32,28),(32,32),(29,35),(20,35),(16,32)],'#48b5b7');line(liquid,[(16,28),(21,27),(27,29),(32,28)],'#a8f1d0')
    for x,y in [(21,31),(27,33),(24,29)]:rect(liquid,(x,y,x+1,y+1),'#b9f1d9')
    poly(cork,[(18,6),(29,6),(30,8),(28,13),(20,13),(18,10)],'#8c5d44',INK);rect(cork,(20,7,27,10),'#cc9c65');line(cork,[(20,7),(27,7)],'#efca8c');rect(cork,(22,9,23,10),'#8c5d44');rect(cork,(26,8,26,9),'#a16f4a')
    for id,img,matid,z in [('glass',glass,'glass',0),('liquid',liquid,'liquid',1),('cork',cork,'cork',2)]:
        p.part(v,id,parent='glass' if id!='glass' else None);glow=lambda c,x,y:(50,160,170) if c[1]>160 else (0,0,0);p.drawing(v,id,img,(24,25),matid,emissive=glow if id=='liquid' else None)
    tracks=[track('glass','glass',24,25,0,y_unused=[])];tracks[0].pop('y_unused')
    tracks[0]['y']=[key(0,25),key(60,22),key(120,25)]
    tracks += [track('liquid','liquid',0,0,1),track('cork','cork',0,0,2)]
    tracks[1]['scaleY']=[key(0,1),key(60,.92),key(120,1)]
    v['animations']=[clip('Float',120,tracks)];return p.save()

def stone():
    p=Package('moss-stone','Moss stone',[mat('stone','Slate',roughness=.9,normalStrength=.9,reflectivity=.25)],native=32,role='terrain',anchor=[16,32],properties={'solid':False,'friction':.85},tileSize=[32,32],repeatable=True)
    v=p.view('Top',32);v['anchor']=[16,16]
    im=canvas(32);normal=canvas(32);surface=canvas(32);emission=canvas(32)
    # Offset joints tessellate into neighbouring tiles; no transparent gutters.
    for y in range(32):
        for x in range(32):
            row=0 if y<16 else 1;xx=(x+(8 if row else 0))%32;edge=min(xx,31-xx,y%16,15-y%16)
            value=(x*17+y*23+x*y*7)%19
            col=(61,76,78) if row else (68,81,82)
            if edge==0:col=(29,42,47)
            elif edge==1:col=(87,101,99) if y%16<8 else (43,58,62)
            elif value<3:col=tuple(v+5 for v in col)
            elif value==7:col=tuple(v-5 for v in col)
            im.putpixel((x,y),col+(255,))
            nx=(-.48 if xx<2 else .48 if xx>29 else 0);ny=(.48 if y%16<2 else -.48 if y%16>13 else 0);nz=math.sqrt(1-nx*nx-ny*ny)
            normal.putpixel((x,y),(int((nx/2+.5)*255),int((ny/2+.5)*255),int((nz/2+.5)*255),255));surface.putpixel((x,y),(220,0,0,255));emission.putpixel((x,y),(0,0,0,255))
    # Moss clusters grow along joints without creating a central repeated symbol.
    for pts,col in [([(1,1),(7,1),(7,3),(4,4),(2,3)],'#57785e'),([(0,14),(4,13),(6,15),(3,17),(0,17)],'#3d6250'),([(23,30),(27,29),(30,30),(31,31),(22,31)],'#557b5c'),([(25,16),(29,16),(30,18),(27,19),(25,18)],'#385c4d')]:poly(im,pts,col)
    line(im,[(2,1),(5,1),(5,2)],'#8a9f70');line(im,[(24,30),(27,30)],'#83a16c')
    p.part(v,'tile');p.drawing(v,'tile',im,(0,0),'stone',maps=(normal,surface,emission));v['animations']=[clip('Still',60,[track('tile','tile')])];return p.save()

def tin_beetle():
    p=Package('tin-beetle','Tin Beetle',[mat('steel','Polished steel',roughness=.18,metalness=.92,reflectivity=1.4,properties={'magnetism':.95}),mat('brass','Brass joints',roughness=.33,metalness=.8),mat('eye','Signal lens',roughness=.1,emission=2.3)],native=56,role='actor',anchor=[28,48],properties={'magnetism':.9,'weight':2.4,'walkSpeed':1.3})
    v=p.view('Front',56);parts={};pivots={}
    # Six independently hinged, complete legs. Copper joints connect steel toes.
    for side in ['l','r']:
        for n,(yy,extend) in enumerate([(22,-4),(29,0),(35,5)]):
            im=canvas(56);x=20;pts=[(x,yy-2),(x-6,yy-3),(x-11,yy+extend),(x-13,yy+extend+6),(x-10,yy+extend+7),(x-8,yy+extend+2),(x-4,yy+1),(x,yy+2)]
            poly(im,pts,'#333f50',INK)
            line(im,[(x-1,yy-1),(x-5,yy-1),(x-9,yy+extend+1),(x-11,yy+extend+5)],'#93adb8',2)
            line(im,[(x-1,yy-2),(x-4,yy-2)],'#e0e9ce')
            ellipse(im,(x-5,yy-3,x-1,yy+1),'#a87849',INK);rect(im,(x-4,yy-2,x-3,yy-1),'#edc17a')
            if side=='r':im=im.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
            id=f'leg-{side}-{n}';parts[id]=im;pivots[id]=(20 if side=='l' else 35,yy)
    # A plated abdomen is drawn over whole hidden leg roots.
    shell=canvas(56)
    poly(shell,[(24,10),(32,10),(38,15),(41,24),(40,33),(35,40),(21,40),(16,34),(14,25),(17,17)],'#495c72',INK)
    poly(shell,[(24,12),(31,12),(36,16),(38,24),(37,33),(33,37),(23,37),(19,32),(17,25),(20,18)],'#7896a5')
    poly(shell,[(24,13),(28,12),(28,35),(23,35),(20,29),(20,22)],'#b2c7c6')
    poly(shell,[(23,14),(25,13),(24,25),(22,28),(21,24)],'#e1e7d1')
    poly(shell,[(30,14),(35,18),(37,25),(35,32),(31,35),(29,33)],'#56718c')
    # Segment seams and rivets establish a rounded mechanical shell.
    for yy in [20,27,34]:
        line(shell,[(18+(yy//10-2),yy),(23,yy+2),(28,yy+2),(35,yy),(38-(yy//10-2),yy-2)],'#334254')
        line(shell,[(21,yy-1),(25,yy+1),(28,yy+1)],'#d4ddcc')
    line(shell,[(28,12),(28,36)],'#344757');line(shell,[(29,14),(29,34)],'#a0b4b7')
    for x,y in [(21,18),(34,18),(20,29),(35,29),(24,36),(32,36)]:
        rect(shell,(x,y,x+1,y+1),'#c49b64');rect(shell,(x,y,x,y),'#f0d49b')
    parts['shell']=shell;pivots['shell']=(28,28)
    # A brass jaw, short antennae and two small emissive instrument lenses.
    head=canvas(56)
    poly(head,[(23,36),(33,36),(38,39),(37,45),(33,48),(23,48),(19,44),(19,40)],'#675044',INK)
    poly(head,[(23,38),(33,38),(35,40),(34,45),(30,46),(23,44),(21,41)],'#bd935d')
    poly(head,[(24,38),(29,38),(29,41),(24,42),(22,40)],'#eed09a')
    line(head,[(23,45),(26,46),(32,46)],'#3b3c40')
    line(head,[(23,38),(19,34),(16,34)],INK,2);line(head,[(33,38),(37,34),(40,34)],INK,2)
    rect(head,(15,33,17,34),'#e2b76a');rect(head,(39,33,41,34),'#e2b76a')
    poly(head,[(23,45),(21,48),(23,50),(26,48)],'#6f8794',INK);poly(head,[(33,45),(35,48),(33,50),(30,48)],'#6f8794',INK)
    line(head,[(22,48),(23,49),(25,48)],'#b5cbc6');line(head,[(34,48),(33,49),(31,48)],'#b5cbc6')
    parts['head']=head;pivots['head']=(28,39)
    eye=canvas(56)
    for x in [24,31]:
        rect(eye,(x-1,40,x+2,43),INK);rect(eye,(x,40,x+1,42),'#75d8d2');rect(eye,(x,40,x,40),'#e1fff0')
    parts['eyes']=eye;pivots['eyes']=(28,39)
    order=[f'leg-{side}-{n}' for n in range(3) for side in ['l','r']]+['shell','head','eyes']
    for z,id in enumerate(order):
        parent=None if id=='shell' else 'head' if id=='eyes' else 'shell'
        p.part(v,id,parent=parent)
        glow=lambda c,x,y:(50,190,180) if c[1]>170 else (0,0,0)
        p.drawing(v,id,parts[id],pivots[id],'eye' if id=='eyes' else 'brass' if id=='head' else 'steel',emissive=glow if id=='eyes' else None,faceted=id=='shell')
    base=[]
    for z,id in enumerate(order):
        parentpivot=(0,0) if id=='shell' else pivots['head'] if id=='eyes' else pivots['shell'];px,py=pivots[id]
        base.append(track(id,id,px-parentpivot[0],py-parentpivot[1],z))
    idle=copy.deepcopy(base);walk=copy.deepcopy(base)
    for tr in idle:
        if tr['part']=='eyes':tr['emission']=[key(0,.65),key(40,1.1),key(80,.65)]
        if tr['part']=='head':tr['rotation']=[key(0,-3),key(40,3),key(80,-3)]
    for tr in walk:
        if tr['part'].startswith('leg'):
            _,side,n=tr['part'].split('-');sg=(1 if side=='l' else -1)*(1 if int(n)%2==0 else -1)
            tr['rotation']=[key(0,-sg*9),key(14,sg*9),key(28,-sg*9)]
        if tr['part']=='shell':tr['y']=[key(0,28),key(7,27),key(14,28),key(21,27),key(28,28)]
    v['animations']=[clip('Idle',80,idle),clip('Walk',28,walk)]
    return p.save()

def reviews(packs):
    bg=(22,31,37,255);sheet=Image.new('RGBA',(960,624),bg);d=ImageDraw.Draw(sheet)
    for n,p in enumerate(packs):
        x=(n%3)*320;y=(n//3)*312;d.text((x+16,y+12),p.doc['name'],fill=(221,234,225))
        art=render(p);size=art.size;scale=min(3,240//max(size))
        large=art.resize((size[0]*scale,size[1]*scale),Image.Resampling.NEAREST);sheet.alpha_composite(large,(x+(320-large.width)//2,y+38))
        sheet.alpha_composite(art,(x+15,y+276-size[1]//2))
        d.text((x+120,y+286),f'{size[0]}px / {len(p.doc["views"])} views',fill=(132,158,159))
    sheet.save(OUT/'contact-sheet.png')
    tile=render(packs[4]);patch=Image.new('RGBA',(128,128))
    for y in range(4):
        for x in range(4):patch.alpha_composite(tile,(x*32,y*32))
    patch.resize((512,512),Image.Resampling.NEAREST).save(OUT/'moss-stone-repeat.png')
    for p in [packs[0],packs[1],packs[5]]:
        views=len(p.doc['views']);frames=6 if p.id=='emberwing' else 4;unit=128 if p.id in ['emberwing','tin-beetle'] else 96
        strip=Image.new('RGBA',(unit*frames,unit*views),bg);draw=ImageDraw.Draw(strip)
        for vi,v in enumerate(p.doc['views']):
            ci=1;length=v['animations'][ci]['length']
            for f in range(frames):
                t=length*f/frames;im=render(p,vi,ci,t);scale=1 if p.id in ['emberwing','tin-beetle'] else 2;im=im.resize((im.width*scale,im.height*scale),Image.Resampling.NEAREST);strip.alpha_composite(im,(f*unit+(unit-im.width)//2,vi*unit+18));draw.text((f*unit+4,vi*unit+3),v['name'] if f==0 else str(round(t)),fill=(150,181,181))
        strip.save(OUT/f'{p.id}-motion.png')
        # Native delivered cels only, no invented interpolation in the review video.
        anim=[]
        for n in range(48):
            im=render(p,0,1,n*length/48);frame=Image.new('RGBA',im.size,bg);frame.alpha_composite(im);anim.append(frame.resize((im.width*4,im.height*4),Image.Resampling.NEAREST).convert('RGB'))
        anim[0].save(OUT/f'{p.id}-preview.gif',save_all=True,append_images=anim[1:],duration=max(20,int(length/60/48*1000)),loop=0,optimize=False)

if __name__=='__main__':
    SOURCE.mkdir(parents=True,exist_ok=True)
    packs=[emberwing(),moss_scout(),lantern(),vial(),stone(),tin_beetle()]
    reviews(packs)
    (OUT/'LICENSE.txt').write_text(LICENSE+'\n')
    print('\n'.join(f'{p.id}: {len(p.doc["views"])} views; {(OUT/(p.id+".sprite")).stat().st_size:,} bytes' for p in packs))
