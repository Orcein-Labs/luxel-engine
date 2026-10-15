"""Bundle the actual modules and source projects into an offline workbench."""
import base64
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
projects = [json.loads((ROOT / f'assets/avatars/native/{name}.json').read_text())
            for name in ['cyan_vtuber', 'ornith_chibi', 'cyan', 'hacker_bird']]
fixtures = 'export const BUILTINS = ' + json.dumps(projects, separators=(',', ':')) + ';\n'
(HERE / 'fixtures.mjs').write_text(fixtures)
code = []
for name in ['runtime.mjs', 'renderer.mjs', 'fixtures.mjs', 'workbench.mjs']:
    lines = (HERE / name).read_text().splitlines()
    code.append('\n'.join(line.removeprefix('export ') for line in lines if not line.startswith('import ')))
script = '\n'.join(code).replace('</script', '<\\/script')
fragment = '<style>\n' + (HERE / 'style.css').read_text() + '\n</style>\n' + (HERE / 'shell.html').read_text()
fragment += '\n<script>\n(() => {\n' + script + '\n})();\n</script>\n'
(HERE / 'workbench-fragment.html').write_text(fragment)
(HERE / 'index.html').write_text('<!doctype html>\n<html lang="en"><head><meta charset="utf-8">'
    '<meta name="viewport" content="width=device-width,initial-scale=1">'
    '<title>Luxel character workbench</title><style>body{margin:0;padding:12px;background:#101922}</style>'
    '</head><body>\n' + fragment + '</body></html>\n')
print('Built offline workbench from shared modules:', len(fragment.encode()), 'bytes')

review_code = []
for name in ['runtime.mjs', 'renderer.mjs', 'fixtures.mjs', 'visual-review.mjs']:
    review_code.append('\n'.join(line.removeprefix('export ') for line in (HERE / name).read_text().splitlines() if not line.startswith('import ')))
reference = 'data:image/png;base64,' + base64.b64encode((ROOT / 'assets/avatars/cyan/source/cyan-source-art-female-v2.png').read_bytes()).decode()
review_script = '\n'.join(review_code).replace('</script', '<\\/script')
review_script += '\nwindow.luxelVisualReview=generateVisualReview(BUILTINS.filter(p=>["cyan_vtuber","ornith_chibi"].includes(p.id)),' + json.dumps(reference) + ');'
(HERE / 'review.html').write_text('<!doctype html><html lang="en"><head><meta charset="utf-8">'
    '<meta name="viewport" content="width=device-width,initial-scale=1"><title>Luxel visual review</title>'
    '<style>body{font:16px system-ui;background:#16232e;color:#e8edf2;margin:24px}figure{margin:0;padding:12px;background:#223443;border-radius:8px}img{display:block;width:100%;height:auto}#luxel-review>figure{max-width:300px;margin-bottom:24px}.review-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:16px}figcaption{padding:12px 0}p{max-width:850px}</style>'
    '</head><body><main id="luxel-review"></main><script>(()=>{\n' + review_script + '\n})();</script></body></html>')
print('Built offline visual review')

guides = [json.loads((HERE / 'guides' / (name + '.json')).read_text()) for name in ['cyan_vtuber', 'ornith_chibi']]
(HERE / 'assembly-guides.mjs').write_text('export const ASSEMBLY_GUIDES = ' + json.dumps(guides, separators=(',', ':')) + ';\n')
assembly_code = []
for name in ['runtime.mjs', 'renderer.mjs', 'assembly.mjs', 'fixtures.mjs', 'assembly-guides.mjs', 'assembly-workbench.mjs']:
    assembly_code.append('\n'.join(line.removeprefix('export ') for line in (HERE / name).read_text().splitlines() if not line.startswith('import ')))
references = [('cyan_vtuber', ROOT / 'assets/avatars/cyan/source/cyan-source-art-female-v2.png'), ('ornith_chibi', HERE / 'references/ornith-mascot.png')]
images = ''.join('<img data-reference="' + name + '" alt="Original design reference" src="data:image/png;base64,' + base64.b64encode(path.read_bytes()).decode() + '">' for name, path in references)
assembly_shell = (HERE / 'assembly-shell.html').read_text().replace('REFERENCE_IMAGES', images)
(HERE / 'assembly.html').write_text('<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Luxel assembly desk</title><style>body{background:#101922;margin:0;padding:12px}' + (HERE / 'assembly.css').read_text() + '</style></head><body>' + assembly_shell + '<script>(()=>{\n' + '\n'.join(assembly_code).replace('</script','<\\/script') + '\n})();</script></body></html>')
print('Built offline assembly desk')

topology_code = []
for name in ['runtime.mjs', 'renderer.mjs', 'assembly.mjs', 'fixtures.mjs', 'assembly-guides.mjs', 'topology.mjs', 'topology-workbench.mjs']:
    topology_code.append('\n'.join(line.removeprefix('export ') for line in (HERE / name).read_text().splitlines() if not line.startswith('import ')))
topology_reference = '<img alt="Supplied Ornith mascot design reference" src="data:image/png;base64,' + base64.b64encode((HERE / 'references/ornith-mascot.png').read_bytes()).decode() + '">'
topology_shell = (HERE / 'topology-shell.html').read_text().replace('TOPOLOGY_REFERENCE', topology_reference)
(HERE / 'topology.html').write_text('<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Luxel garment opening editor</title><style>body{background:#101922;margin:0;padding:12px}' + (HERE / 'topology.css').read_text() + '</style></head><body>' + topology_shell + '<script>(async()=>{\n' + '\n'.join(topology_code).replace('</script','<\\/script') + '\n})();</script></body></html>')
print('Built offline garment opening editor')

reference_code = []
for name in ['runtime.mjs', 'renderer.mjs', 'reference-assets.mjs', 'reference-separation.mjs', 'reference-workbench.mjs']:
    reference_code.append('\n'.join(line.removeprefix('export ') for line in (HERE / name).read_text().splitlines() if not line.startswith('import ')))
original_data = 'data:image/webp;base64,' + base64.b64encode((HERE / 'references/ornith-front-original.webp').read_bytes()).decode()
reference_shell = (HERE / 'reference-shell.html').read_text().replace('REFERENCE_SOURCE', original_data)
(HERE / 'reference.html').write_text('<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Luxel original Ornith separation</title><style>' + (HERE / 'reference-style.css').read_text() + '</style></head><body>' + reference_shell + '<script>(async()=>{\n' + '\n'.join(reference_code).replace('</script', '<\\/script') + '\n})();</script></body></html>')
print('Built offline original-reference separation workspace')
