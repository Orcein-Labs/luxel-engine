"""Original vector control sources for the native layered-character profile."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INK = "#172738"


def ellipse(x, y, rx, ry):
    k = 0.55228475
    return [['M', x+rx, y], ['C', x+rx, y+k*ry, x+k*rx, y+ry, x, y+ry],
            ['C', x-k*rx, y+ry, x-rx, y+k*ry, x-rx, y],
            ['C', x-rx, y-k*ry, x-k*rx, y-ry, x, y-ry],
            ['C', x+k*rx, y-ry, x+rx, y-k*ry, x+rx, y], ['Z']]


def channel(parameter, prop, gain):
    return {'parameter': parameter, 'property': prop, 'gain': gain}


def project(subject):
    params = [('head', 'Head tilt', -1, 1), ('body', 'Body lean', -1, 1),
              ('gazeX', 'Look left / right', -1, 1), ('gazeY', 'Look up / down', -1, 1),
              ('blinkL', 'Left blink', 0, 1), ('blinkR', 'Right blink', 0, 1),
              ('speech', 'Mouth / beak open', 0, 1), ('sway', 'Secondary sway', -1, 1),
              ('typing', 'Typing gesture', 0, 1)]
    return {'schema': 'luxel.layered-character/v1', 'id': subject,
            'label': 'Cyan — vector study' if subject == 'cyan' else 'Hacker bird — vector study',
            'canvas': [600, 700],
            'parameters': [{'id': i, 'label': l, 'min': a, 'max': b, 'default': 0} for i, l, a, b in params],
            'nodes': [{'id': 'body', 'parent': None, 'pivot': [300, 600], 'channels': [channel('body', 'rotate', 3)]},
                      {'id': 'head', 'parent': 'body', 'pivot': [300, 420], 'channels': [channel('head', 'rotate', 9), channel('head', 'x', 8)]},
                      {'id': 'prop', 'parent': None, 'pivot': [300, 600], 'channels': []}],
            'parts': [], 'clips': [],
            'bindings': {'gazeX': 'gazeX', 'gazeY': 'gazeY', 'blinkLeft': 'blinkL', 'blinkRight': 'blinkR',
                         'speech': 'speech', 'head': 'head', 'body': 'body', 'secondary': 'sway'},
            'provenance': {'kind': 'original-authored-vector-control', 'date': '2026-10-09',
                           'quality': 'MVP control artwork; no professional-quality approval; not Live2D'}}


def part(p, name, path, color, order, node='head', pivot=(300, 320), channels=(), mask=None, stroke=INK, width=3):
    value = {'id': name, 'label': name.replace('_', ' ').capitalize(), 'node': node,
             'pivot': list(pivot), 'order': order, 'visible': True, 'fill': color, 'stroke': stroke,
             'strokeWidth': width, 'path': path, 'channels': list(channels), 'deforms': [], 'mask': mask}
    p['parts'].append(value)
    return value


def eyes(p, y, spread, size):
    for suffix, x, blink in [('left', 300-spread, 'blinkL'), ('right', 300+spread, 'blinkR')]:
        blink_channel = channel(blink, 'scaleY', -0.96)
        part(p, f'eye_{suffix}', ellipse(x, y, size, 20), '#fffaf1', 40,
             pivot=(x, y), channels=[blink_channel])
        part(p, f'iris_{suffix}', ellipse(x, y, 11, 15), '#32a8ab', 41,
             pivot=(x, y), channels=[channel('gazeX', 'x', 8), channel('gazeY', 'y', 5), blink_channel], mask=f'eye_{suffix}')
        part(p, f'pupil_{suffix}', ellipse(x, y+1, 5, 10), INK, 42,
             pivot=(x, y), channels=[channel('gazeX', 'x', 8), channel('gazeY', 'y', 5), blink_channel], mask=f'eye_{suffix}', width=0)
        part(p, f'glint_{suffix}', ellipse(x-3, y-5, 3, 4), '#ffffff', 43,
             pivot=(x, y), channels=[channel('gazeX', 'x', 8), channel('gazeY', 'y', 5), blink_channel], mask=f'eye_{suffix}', width=0)
        part(p, f'lash_{suffix}', [['M', x-size-3, y-7], ['Q', x, y-29, x+size+3, y-7]], 'none', 44,
             pivot=(x, y), channels=[blink_channel], width=4)
        part(p, f'brow_{suffix}', [['M', x-size, y-38], ['Q', x, y-46, x+size, y-39]], 'none', 45, width=4)


def human():
    p = project('cyan')
    p['parameters'] = [parameter for parameter in p['parameters'] if parameter['id'] != 'typing']
    part(p, 'hair_back', [['M', 155, 245], ['C', 142, 107, 453, 105, 448, 260],
         ['L', 466, 488], ['Q', 398, 529, 351, 476], ['L', 249, 477], ['Q', 185, 519, 135, 481], ['Z']],
         '#23354e', 0, pivot=(300, 210), channels=[channel('sway', 'rotate', 2)])
    part(p, 'neck', [['M', 259, 374], ['L', 342, 374], ['L', 352, 484], ['Q', 300, 520, 246, 484], ['Z']], '#e9b19c', 5, 'body')
    part(p, 'coat', [['M', 240, 470], ['Q', 163, 472, 129, 519], ['L', 89, 672], ['Q', 300, 698, 512, 672],
         ['L', 474, 520], ['Q', 436, 474, 359, 470], ['Q', 300, 513, 240, 470], ['Z']], '#e7edf0', 8, 'body', width=4)
    part(p, 'shirt', [['M', 249, 468], ['Q', 300, 493, 350, 468], ['L', 380, 681], ['L', 218, 681], ['Z']], '#254453', 9, 'body')
    part(p, 'collar_left', [['M', 246, 464], ['L', 279, 498], ['L', 255, 575], ['L', 202, 494], ['Z']], '#ffffff', 10, 'body')
    part(p, 'collar_right', [['M', 354, 464], ['L', 322, 498], ['L', 346, 575], ['L', 397, 494], ['Z']], '#ffffff', 10, 'body')
    part(p, 'coat_seam_left', [['M', 184, 552], ['L', 164, 671]], 'none', 11, 'body', stroke='#8198a4', width=2)
    part(p, 'coat_seam_right', [['M', 416, 552], ['L', 437, 671]], 'none', 11, 'body', stroke='#8198a4', width=2)
    part(p, 'badge', [['M', 393, 562], ['L', 423, 562], ['L', 423, 593], ['L', 393, 593], ['Z']], '#57c9bc', 12, 'body', width=2)
    part(p, 'ear_left', ellipse(182, 330, 20, 33), '#f1c1aa', 18)
    part(p, 'ear_right', ellipse(418, 330, 20, 33), '#f1c1aa', 18)
    part(p, 'face', [['M', 192, 245], ['C', 193, 178, 409, 178, 409, 245], ['L', 408, 339],
         ['C', 398, 390, 343, 438, 301, 446], ['C', 258, 438, 201, 391, 192, 339], ['Z']], '#f4cbb5', 20, width=3.5)
    part(p, 'face_light', [['M', 212, 272], ['Q', 250, 232, 285, 258], ['L', 274, 365],
         ['Q', 227, 369, 212, 337], ['Z']], '#f9dcc5', 21, width=0)
    part(p, 'cheek_left', ellipse(222, 357, 19, 7), '#e9a79f', 25, width=0)
    part(p, 'cheek_right', ellipse(378, 357, 19, 7), '#e9a79f', 25, width=0)
    eyes(p, 316, 59, 30)
    part(p, 'nose', [['M', 300, 326], ['L', 293, 351], ['Q', 300, 357, 306, 351]], 'none', 46, stroke='#b87669', width=2)
    mouth = part(p, 'mouth', [['M', 278, 383], ['Q', 300, 390, 322, 383], ['Q', 300, 390, 278, 383], ['Z']], '#833e50', 47, width=2)
    count = sum(len(c)-1 for c in mouth['path']); positive = [0]*count
    positive[3], positive[7] = -3, 17
    mouth['deforms'] = [{'parameter': 'speech', 'negative': [0]*count, 'positive': positive}]
    part(p, 'hair_cap', [['M', 171, 277], ['C', 157, 175, 220, 147, 294, 152], ['C', 393, 138, 447, 204, 426, 277],
         ['Q', 406, 242, 369, 225], ['Q', 339, 263, 297, 278], ['L', 315, 229],
         ['Q', 257, 265, 210, 261], ['L', 190, 298], ['Z']], '#263950', 60, width=4)
    part(p, 'hair_shine', [['M', 196, 226], ['Q', 241, 176, 303, 181], ['Q', 264, 194, 235, 226],
         ['L', 251, 213], ['Q', 221, 242, 196, 243], ['Z']], '#53738c', 61, width=0)
    part(p, 'hair_left', [['M', 180, 254], ['Q', 163, 338, 190, 427], ['L', 217, 459],
         ['Q', 185, 424, 206, 273], ['Z']], '#263950', 62, pivot=(190, 260), channels=[channel('sway', 'rotate', 5)])
    part(p, 'hair_right', [['M', 417, 252], ['Q', 442, 339, 412, 443], ['L', 386, 469],
         ['Q', 411, 389, 394, 267], ['Z']], '#263950', 62, pivot=(409, 260), channels=[channel('sway', 'rotate', -5)])
    part(p, 'hair_clip', [['M', 391, 262], ['L', 413, 270], ['L', 409, 285], ['L', 387, 276], ['Z']], '#65d9ce', 64, width=2)
    return p


def bird():
    p = project('hacker_bird')
    part(p, 'hoodie', [['M', 206, 399], ['Q', 132, 421, 118, 503], ['L', 116, 617],
         ['Q', 299, 677, 485, 616], ['L', 476, 503], ['Q', 457, 414, 394, 398], ['Z']], '#283b50', 5, 'body', width=5)
    part(p, 'hood_rim', [['M', 190, 403], ['Q', 300, 474, 410, 403], ['Q', 426, 435, 402, 461],
         ['Q', 300, 504, 193, 458], ['Q', 173, 436, 190, 403], ['Z']], '#182a3b', 9, 'body', width=4)
    for suffix, x in [('left', 253), ('right', 350)]:
        part(p, f'cord_{suffix}', [['M', x, 469], ['L', x-3, 535]], 'none', 11, 'body', stroke='#e6ebdf', width=6)
    part(p, 'crest_back', [['M', 269, 205], ['Q', 268, 117, 346, 79], ['Q', 361, 142, 329, 180],
         ['Q', 375, 126, 394, 143], ['Q', 395, 190, 349, 216], ['Z']], '#ffe7a2', 15,
         pivot=(305, 200), channels=[channel('sway', 'rotate', 8)], width=4)
    part(p, 'face', [['M', 167, 279], ['C', 151, 184, 247, 165, 302, 175],
         ['C', 390, 160, 450, 223, 437, 318], ['C', 434, 398, 367, 451, 302, 454],
         ['C', 228, 444, 165, 382, 167, 279], ['Z']], '#fff5db', 20, width=5)
    part(p, 'face_light', ellipse(264, 286, 72, 78), '#fffdf4', 21, width=0)
    part(p, 'cheek_left', ellipse(217, 355, 24, 15), '#f7ca78', 25, width=0)
    part(p, 'cheek_right', ellipse(387, 355, 24, 15), '#f7ca78', 25, width=0)
    eyes(p, 307, 59, 27)
    part(p, 'beak_interior', [['M', 272, 349], ['Q', 300, 329, 329, 352], ['L', 315, 397],
         ['Q', 301, 411, 286, 397], ['Z']], '#793e35', 48, width=3)
    part(p, 'beak_lower', [['M', 274, 366], ['Q', 300, 382, 327, 366], ['Q', 316, 406, 300, 407],
         ['Q', 285, 402, 274, 366], ['Z']], '#f39556', 49, pivot=(300, 365),
         channels=[channel('speech', 'y', 15), channel('speech', 'scaleY', 0.15)], width=4)
    part(p, 'beak_upper', [['M', 272, 358], ['Q', 277, 326, 300, 327], ['Q', 325, 326, 330, 357],
         ['Q', 315, 379, 300, 383], ['Q', 282, 373, 272, 358], ['Z']], '#f9b071', 50, width=4)
    part(p, 'beak_shine', [['M', 284, 347], ['Q', 288, 334, 297, 337]], 'none', 51, stroke='#ffe3bc', width=5)
    part(p, 'wing_left', [['M', 163, 469], ['Q', 104, 476, 94, 544], ['Q', 90, 585, 147, 595],
         ['L', 220, 564], ['L', 210, 526], ['Z']], '#344a60', 70, 'body', pivot=(164, 481), channels=[channel('typing', 'rotate', 4)], width=5)
    part(p, 'wing_right', [['M', 437, 470], ['Q', 494, 478, 506, 545], ['Q', 513, 587, 454, 596],
         ['L', 382, 565], ['L', 389, 527], ['Z']], '#344a60', 70, 'body', pivot=(437, 481), channels=[channel('typing', 'rotate', -4)], width=5)
    part(p, 'laptop_base', [['M', 149, 610], ['L', 451, 610], ['L', 500, 641], ['Q', 507, 654, 481, 659],
         ['L', 118, 659], ['Q', 93, 652, 109, 640], ['Z']], '#63778a', 90, 'prop', width=5)
    part(p, 'paw_left', ellipse(204, 634, 30, 14), '#fff2d4', 104, 'body', pivot=(204, 634), channels=[channel('typing', 'y', 4)], width=4)
    part(p, 'paw_right', ellipse(398, 634, 30, 14), '#fff2d4', 104, 'body', pivot=(398, 634), channels=[channel('typing', 'y', -4)], width=4)
    part(p, 'laptop_lid', [['M', 137, 502], ['Q', 135, 489, 151, 488], ['L', 424, 488], ['Q', 439, 488, 441, 502],
         ['L', 463, 635], ['L', 157, 635], ['Z']], '#24394e', 100, 'prop', width=5)
    part(p, 'laptop_edge', [['M', 153, 500], ['L', 424, 500], ['L', 444, 622]], 'none', 101, 'prop', stroke='#8196aa', width=3)
    part(p, 'code_left', [['M', 279, 543], ['L', 255, 565], ['L', 281, 587]], 'none', 102, 'prop', stroke='#c8f3e9', width=7)
    part(p, 'code_right', [['M', 329, 543], ['L', 353, 565], ['L', 327, 587]], 'none', 102, 'prop', stroke='#c8f3e9', width=7)
    part(p, 'code_slash', [['M', 315, 538], ['L', 299, 591]], 'none', 102, 'prop', stroke='#65d9ce', width=6)
    return p


def add_clip(p):
    tracks = {'head': [[0, 0], [1.5, -0.6], [3, 0.65], [4.5, -0.25], [6, 0]],
              'body': [[0, 0], [2, 0.6], [4, -0.6], [6, 0]],
              'sway': [[0, 0], [1.5, 0.8], [3, -0.8], [4.5, 0.5], [6, 0]],
              'blinkL': [[0, 0], [1, 0], [1.15, 1], [1.3, 0], [3.8, 0], [4, 1], [4.2, 0], [6, 0]],
              'blinkR': [[0, 0], [1, 0], [1.15, 1], [1.3, 0], [5, 0], [5.15, 1], [5.3, 0], [6, 0]],
              'gazeX': [[0, 0], [2, -0.8], [4, 0.8], [6, 0]],
              'speech': [[0, 0], [0.3, 0.9], [0.6, 0.1], [0.9, 0.7], [1.2, 0], [2, 0], [2.4, 1], [2.7, 0.1], [3, 0.8], [3.3, 0], [6, 0]],
              'typing': [[0, 0], [2, 0], [2.3, 1], [2.6, 0], [2.9, 1], [3.2, 0], [3.5, 1], [4, 0], [6, 0]]}
    p['clips'] = [{'id': 'demo', 'duration': 6, 'loop': True,
                   'tracks': [{'parameter': key, 'keys': keys} for key, keys in tracks.items()
                              if key in {parameter['id'] for parameter in p['parameters']}]}]


if __name__ == '__main__':
    destination = ROOT / 'assets/avatars/native'
    destination.mkdir(parents=True, exist_ok=True)
    for p in [human(), bird()]:
        add_clip(p)
        (destination / f"{p['id']}.json").write_text(json.dumps(p, indent=2) + '\n')
        print(p['id'], len(p['parts']), 'editable parts')
