#!/usr/bin/env python3
"""Rebuild the pixel-grid room plate and editable .sprite from saved generation.

The generated composition is preserved; conversion is a native-grid area sample
and a 40-colour stone/bronze palette without dithering. No pixels are drawn by code.
"""
from pathlib import Path
from PIL import Image, ImageOps
import io, json, zipfile

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / 'web/art-source'
OUT = ROOT / 'web/examples'
SIZE = (640, 360)

def png(image):
    buf = io.BytesIO()
    image.save(buf, format='PNG', optimize=True)
    return buf.getvalue()

def main():
    original = Image.open(ART / 'obsidian-hall-source.png').convert('RGB')
    sampled = ImageOps.fit(original, SIZE, method=Image.Resampling.BOX)
    # Quantization retains both the broad stone ramp and the fine bronze relief.
    indexed = sampled.quantize(colors=40, method=Image.Quantize.MEDIANCUT,
                               dither=Image.Dither.NONE)
    color = indexed.convert('RGBA')
    color.save(OUT / 'obsidian-hall.png')
    color.resize((1280, 720), Image.Resampling.NEAREST).save(OUT / 'obsidian-hall-preview.png')
    colors = ['#%02x%02x%02x' % tuple(indexed.getpalette()[i:i+3])
              for i in range(0, 40*3, 3)]
    (ART / 'obsidian-hall-palette.json').write_text(json.dumps({
        'name':'Obsidian stone and aged bronze', 'colors':colors,
        'sourceSize': list(original.size), 'nativeSize':list(SIZE),
        'conversion':'Centered aspect-preserving area sample; 40-color median cut, no dithering'
    }, indent=2)+'\n')
    images = {
        'room/color.png': color,
        'room/normal.png': Image.new('RGBA', SIZE, (128,128,255,255)),
        'room/surface.png': Image.new('RGBA', SIZE, (255,255,255,255)),
        'room/emission.png': Image.new('RGBA', SIZE, (0,0,0,255)),
    }
    provenance = ('Original composition generated with OpenAI built-in image_gen for Sprite Lab, '
                  '2026-11-09. Prompt and full generated source retained in web/art-source/. '
                  'Native 640x360 conversion by tools/make_obsidian_hall.py. '
                  'No extracted game artwork. Project artwork dedicated under CC0 1.0.')
    document = {
        'format':'sprite-lab', 'version':0, 'name':'Obsidian hall',
        'ticksPerSecond':60, 'required':['core-0'],
        'role':'terrain', 'showcaseOnly':True, 'nativeSize':list(SIZE),
        'anchor':[320,360], 'repeatable':False,
        'properties':{'environment':True,'friction':0.85},
        'provenance':provenance,
        'materials':[{
            'id':'stone','name':'Obsidian and bronze', 'roughness':0.96,
            'metalness':0.0,'transmission':0.0,'emission':0.0,
            'normalStrength':0.0,'reflectivity':0.15
        }],
        'views':[{
            'id':'room','name':'Room',
            'canvas':{'x':0,'y':0,'width':640,'height':360},
            'parts':[{'id':'architecture','name':'Architecture'}],
            'images':[{
                'id':'room','name':'Room','x':0,'y':0,'material':'stone',
                **{channel:f'room/{channel}.png' for channel in ['color','normal','surface','emission']}
            }],
            'animations':[{
                'id':'still','name':'Still','length':60,'looping':True,
                'tracks':[{'part':'architecture','image':[{'tick':0,'value':'room','ease':'step'}]}]
            }]
        }]
    }
    entries = {name:png(im) for name,im in images.items()}
    entries['sprite.json'] = (json.dumps(document,indent=2)+'\n').encode()
    entries['LICENSE.txt'] = (provenance+'\nhttps://creativecommons.org/publicdomain/zero/1.0/\n').encode()
    entries['SOURCE.txt'] = b'Generated source and prompt: web/art-source/obsidian-hall-*\nRebuild: python3 tools/make_obsidian_hall.py\n'
    with zipfile.ZipFile(OUT / 'obsidian-hall.sprite','w') as z:
        for name,data in sorted(entries.items()):
            info=zipfile.ZipInfo(name,(1980,1,1,0,0,0))
            info.compress_type=zipfile.ZIP_STORED if name.endswith('.png') else zipfile.ZIP_DEFLATED
            z.writestr(info,data)
    (ART / 'obsidian-hall-manifest.json').write_text(json.dumps(document,indent=2)+'\n')
    assert color.size == SIZE
    assert color.getextrema()[3] == (255,255)
    assert len(color.getcolors()) <= 40
    print('Built Obsidian hall: 640x360, 40 colours, fully opaque, flat normals, no emission.')

if __name__ == '__main__':
    main()
