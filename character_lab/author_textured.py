"""Bind unchanged image-tool PNG atlases to explicit native layer recipes."""
import base64
import hashlib
import json
import struct
from pathlib import Path
from author_controls import project, part, ellipse, channel

ROOT = Path(__file__).resolve().parents[1]


def rectangle(r):
    x, y, w, h = r
    return [['M', x, y], ['L', x+w, y], ['L', x+w, y+h], ['L', x, y+h], ['Z']]


def begin(identity, label, path, canvas):
    p = project(identity)
    data = (ROOT / path).read_bytes()
    p.update(schema='luxel.layered-character/v2', label=label, canvas=canvas)
    p['textures'] = [{'id': 'atlas', 'data': 'data:image/png;base64,' + base64.b64encode(data).decode(),
                      'size': list(struct.unpack('>II', data[16:24]))}]
    p['provenance'] = {'kind': 'image-tool-atlas-with-authored-layer-recipe', 'source': path,
                       'sha256': hashlib.sha256(data).hexdigest(), 'date': '2026-10-09',
                       'quality': 'Native raster pilot, no professional sign-off; not Cubism',
                       'method': 'Unchanged PNG; explicit runtime source regions, no offline pixel editing'}
    p['nodes'][0]['channels'] = [channel('body', 'rotate', 1.2)]
    p['nodes'][1]['pivot'] = [300, 350]
    p['nodes'][1]['channels'] = [channel('head', 'rotate', 4), channel('head', 'x', 3)]
    return p


def image_part(p, identity, source, rect, order, node='head', channels=(), opacity=1, mask=None):
    value = part(p, identity, rectangle(rect), 'none', order, node,
                 pivot=(rect[0]+rect[2]/2, rect[1]), channels=channels, mask=mask, stroke='none', width=0)
    value['raster'] = {'texture': 'atlas', 'source': source, 'rect': rect}
    value['opacity'] = opacity
    return value


def motion(p):
    blink = [[0,0],[1.2,0],[1.28,1],[1.32,1],[1.48,0],[5.1,0],[5.18,1],[5.22,1],[5.38,0],[9.2,0],[9.28,1],[9.32,1],[9.48,0],[12,0]]
    tracks = {
        'head': [[0,0],[1.8,0],[2.8,-.30],[4.2,-.30],[5.6,.23],[7.5,.23],[9.0,-.1],[10.8,-.1],[12,0]],
        'body': [[0,0],[3,.16],[6,-.12],[9,.10],[12,0]],
        'sway': [[0,0],[2.2,.15],[4.8,-.12],[7.5,.10],[10,-.08],[12,0]],
        'gazeX': [[0,0],[1.8,0],[1.94,-.42],[4.4,-.42],[4.54,.38],[8,.38],[8.14,0],[12,0]],
        'gazeY': [[0,0],[4.4,0],[4.54,-.12],[8,-.12],[8.14,0],[12,0]],
        'blinkL': blink, 'blinkR': blink,
        'speech': [[0,0],[.8,0],[.91,.55],[1.08,.12],[1.22,.7],[1.42,.15],[1.58,.4],[1.76,0],
                   [3.3,0],[3.42,.5],[3.56,.12],[3.72,.75],[3.9,.18],[4.08,.35],[4.3,0],[12,0]],
        'typing': [[0,0],[6,0],[6.15,1],[6.3,0],[6.45,1],[6.6,0],[6.75,1],[6.9,0],[12,0]],
    }
    ids = {v['id'] for v in p['parameters']}
    p['clips'] = [{'id': 'demo', 'duration': 12, 'loop': True,
                   'tracks': [{'parameter': k, 'keys': v, 'interpolation': 'smooth' if k in ['head','body','sway'] else 'linear'}
                              for k, v in tracks.items() if k in ids]}]


def human():
    p = begin('cyan_vtuber', 'Cyan — VTuber art pilot', 'assets/avatars/cyan/source/cyan-parts-atlas.png', [600,800])
    p['parameters'] = [v for v in p['parameters'] if v['id'] != 'typing']
    image_part(p, 'hair_back', [670,5,280,420], [167,70,266,405], 0,
               channels=[channel('sway','rotate',1)])
    image_part(p, 'neck_bridge', [475,347,72,22], [271,333,58,45], 4, 'body')
    image_part(p, 'torso', [0,15,367,373], [90,337,420,415], 5, 'body')
    image_part(p, 'face', [375,20,285,370], [211,130,178,231], 20)
    for side, x, blink, eye, iris, closed in [
        ('left',226,'blinkL',[25,462,307,205],[741,500,150,170],[25,696,307,185]),
        ('right',311,'blinkR',[370,462,267,205],[1020,500,145,178],[365,696,274,190])]:
        # Eye whites are authored vector masks beneath the generated eye borders.
        part(p, 'eye_white_'+side, ellipse(x+32,263,24,10), '#fff8f5', 39,
             pivot=(x+32,263), channels=[channel(blink,'opacity',-1)], stroke='none', width=0)
        image_part(p, 'iris_'+side, iris, [x+20,250,25,28], 40,
                   channels=[channel('gazeX','x',5),channel('gazeY','y',3),channel(blink,'opacity',-1)], mask='eye_white_'+side)
        image_part(p, 'eye_'+side, eye, [x,237,65,43], 41, channels=[channel(blink,'opacity',-1)])
        image_part(p, 'closed_'+side, closed, [x,237,65,43], 42, channels=[channel(blink,'opacity',1)], opacity=0)
    image_part(p, 'mouth_closed', [700,735,235,140], [281,295,38,22], 47, channels=[channel('speech','opacity',-1)])
    image_part(p, 'mouth_open', [1025,720,180,178], [285,293,30,30], 48, channels=[channel('speech','opacity',1)], opacity=0)
    image_part(p, 'hair_front', [951,30,303,388], [190,75,226,290], 60)
    image_part(p, 'hair_left', [20,882,290,362], [173,275,110,280], 61, channels=[channel('sway','rotate',2)])
    image_part(p, 'hair_right', [365,882,270,360], [329,276,100,274], 61, channels=[channel('sway','rotate',-2)])
    motion(p)
    return p


def bird():
    p = begin('ornith_chibi', 'Ornith — chibi art pilot', 'assets/avatars/ornith/source/ornith-parts-atlas.png', [600,700])
    p['parameters'].append({'id':'shades','label':'Hacker sunglasses','min':0,'max':1,'default':1})
    image_part(p, 'hoodie', [15,130,309,220], [147,395,306,220], 5, 'body')
    # Generated head includes the crest: do not claim independent crest motion.
    image_part(p, 'face', [336,40,291,325], [157,101,292,325], 20)
    for side,x,blink,eye,iris,closed in [
        ('left',218,'blinkL',[70,480,170,160],[700,485,165,160],[70,810,160,95]),
        ('right',334,'blinkR',[385,480,175,165],[1020,485,165,160],[405,810,165,95])]:
        center = x + (30 if side == 'left' else 22)
        part(p, 'eye_window_'+side, ellipse(center,313,20,19), 'none', 39,
             pivot=(center,313), stroke='none', width=0)
        image_part(p, 'eye_'+side, eye, [x,281,54,54], 40, channels=[channel(blink,'opacity',-1)])
        image_part(p, 'iris_'+side, [700,485,165,160], [center-20,291,42,44], 41,
                   channels=[channel('gazeX','x',4),channel('gazeY','y',3),channel(blink,'opacity',-1)], mask='eye_window_'+side)
        image_part(p, 'closed_'+side, closed, [x+2,301,50,24], 42, channels=[channel(blink,'opacity',1)], opacity=0)
    part(p, 'beak_interior', ellipse(300,363,14,1), '#402015', 46,
         pivot=(300,363), channels=[channel('speech','scaleY',8)], stroke='none', width=0)
    image_part(p, 'beak_lower', [1040,830,115,85], [280,352,40,16], 47,
               channels=[channel('speech','y',5),channel('speech','scaleY',.8)])
    image_part(p, 'beak_upper', [720,780,135,130], [274,329,52,44], 48)
    image_part(p, 'sunglasses', [633,1040,334,145], [208,267,205,89], 60,
               channels=[channel('shades','opacity',1)])
    image_part(p, 'laptop', [945,140,300,203], [140,468,340,225], 90, 'prop')
    image_part(p, 'paw_left', [55,1040,205,155], [187,638,65,42], 95, 'prop', channels=[channel('typing','y',3)])
    image_part(p, 'paw_right', [385,1040,200,155], [368,638,65,42], 95, 'prop', channels=[channel('typing','y',-3)])
    motion(p)
    return p


if __name__ == '__main__':
    for p in [human(),bird()]:
        (ROOT / f"assets/avatars/native/{p['id']}.json").write_text(json.dumps(p,indent=2)+'\n')
        print(p['id'], len(p['parts']), 'layers')
