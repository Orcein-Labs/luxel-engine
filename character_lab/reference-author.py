"""Prepare unchanged source pixels and native vector mask proposals."""
from pathlib import Path
from collections import deque, defaultdict
import json, hashlib, math, base64
from PIL import Image

HERE=Path(__file__).resolve().parent
src=HERE/'references/ornith-front-original.webp'
im=Image.open(src).convert('RGB'); w,h=im.size
png=HERE/'references/ornith-front-decoded.png'; im.save(png)
assert Image.open(png).convert('RGB').tobytes()==im.tobytes()
pixels=im.load()

def simplify(points,epsilon=1.5):
    if len(points)<3:return points
    a,b=points[0],points[-1]; dx,dy=b[0]-a[0],b[1]-a[1]; length=math.hypot(dx,dy)
    ds=[abs(dy*(p[0]-a[0])-dx*(p[1]-a[1]))/length if length else math.dist(a,p) for p in points]
    j=max(range(len(ds)),key=ds.__getitem__)
    return simplify(points[:j+1],epsilon)[:-1]+simplify(points[j:],epsilon) if ds[j]>epsilon else [a,b]

def loops(cells):
    edges=defaultdict(list)
    for x,y in cells:
        for absent,a,b in [((x,y-1),(x,y),(x+1,y)),((x+1,y),(x+1,y),(x+1,y+1)),((x,y+1),(x+1,y+1),(x,y+1)),((x-1,y),(x,y+1),(x,y))]:
            if absent not in cells:edges[a].append(b)
    paths=[]
    while edges:
        start=next(iter(edges)); p=start; points=[p]
        while True:
            q=edges[p].pop()
            if not edges[p]:del edges[p]
            p=q;points.append(p)
            if p==start:break
        area=sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(points,points[1:]))/2
        if abs(area)<10:continue
        halfway=len(points)//2
        points=simplify(points[:halfway+1])+simplify(points[halfway:])[1:-1]
        paths.extend([['M',*points[0]]]+[['L',*p]for p in points[1:]]+[['Z']])
    return paths

def polygon(points):return [['M',*points[0]]]+[['L',*p]for p in points[1:]]+[['Z']]
def ellipse(cx,cy,rx,ry):return polygon([[round(cx+rx*math.cos(i*math.tau/64),2),round(cy+ry*math.sin(i*math.tau/64),2)]for i in range(64)])
def inside(poly,x,y):
    result=False
    for a,b in zip(poly,poly[1:]+poly[:1]):
        if (a[1]>y)!=(b[1]>y) and x<(b[0]-a[0])*(y-a[1])/(b[1]-a[1])+a[0]:result=not result
    return result

# The closed outer ink separates the character from the connected gray backdrop.
outside={(0,0)}; q=deque(outside)
while q:
    x,y=q.popleft()
    for a,b in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)]:
        if 0<=a<w and 0<=b<h and (a,b)not in outside and max(pixels[a,b])>=24:
            outside.add((a,b));q.append((a,b))
silhouette=loops({(x,y)for y in range(h)for x in range(w)if(x,y)not in outside})
head=[[184,458],[201,414],[225,369],[260,324],[303,285],[355,251],[406,217],[456,181],[502,140],[544,90],[581,45],[608,29],[637,29],[659,42],[671,65],[677,97],[675,127],[666,158],[653,185],[683,156],[702,155],[715,170],[719,190],[716,214],[704,241],[689,262],[722,283],[753,309],[783,339],[807,374],[828,416],[844,456],[854,510],[860,563],[860,616],[852,659],[836,692],[812,718],[778,745],[738,773],[693,799],[645,824],[596,849],[552,875],[512,892],[474,875],[430,851],[381,827],[335,803],[292,779],[253,755],[217,728],[193,704],[177,676],[168,646],[165,609],[169,562],[176,513]]
glasswindow=[[150,452],[206,425],[283,419],[359,423],[425,435],[451,450],[471,475],[496,468],[527,469],[553,477],[563,451],[597,432],[681,419],[767,424],[818,437],[842,456],[874,455],[880,496],[840,504],[833,558],[820,606],[792,636],[747,652],[652,652],[589,639],[567,618],[550,582],[549,512],[469,512],[460,579],[442,619],[410,641],[351,651],[280,647],[230,638],[207,617],[190,580],[183,517],[151,503]]
glass=loops({(x,y)for y in range(415,655)for x in range(150,881)if max(pixels[x,y])<32 and inside(glasswindow,x+.5,y+.5) and not(300<x<410 and 488<y<608)and not(613<x<723 and 488<y<608)and not(435<x<582 and 520<y<730)})
regions=[('hood_body','Hoodie body',polygon([[0,0],[w,0],[w,h],[0,h]])),
 ('hood_lip','Front hood / collar',polygon([[119,728],[166,741],[225,768],[304,800],[390,844],[464,886],[512,906],[561,881],[640,835],[723,794],[805,754],[864,728],[902,741],[908,786],[876,815],[810,831],[743,852],[675,888],[610,925],[552,950],[512,954],[470,943],[410,919],[348,886],[281,852],[216,833],[147,817],[111,792]])),
 ('head_face','Head and crest',polygon(head)),
 ('beak_shadow','Beak shadow (no authored jaw)',polygon([[447,608],[469,593],[530,590],[573,609],[585,641],[574,675],[554,702],[530,718],[505,721],[478,710],[459,686],[447,654]])),
 ('beak_upper','Visible beak (single painted form)',polygon([[461,559],[474,540],[496,531],[521,535],[542,548],[557,569],[569,595],[577,621],[572,640],[553,654],[535,674],[521,693],[506,702],[492,695],[479,677],[465,663],[449,652],[438,635],[432,615],[441,600],[448,578]])),
 ('eye_left','Left pupil',ellipse(355,546,44.5,51)),('eye_right','Right pupil',ellipse(668,546,44.5,51)),
 ('glasses','Glasses frame',glass),
 ('string_left','Left drawstring',polygon([[380,835],[398,835],[411,850],[410,927],[420,938],[420,966],[410,980],[408,1005],[392,1013],[376,1007],[372,980],[358,968],[357,943],[367,930],[370,856]])),
 ('string_right','Right drawstring',polygon([[617,836],[635,835],[649,849],[652,926],[665,940],[665,965],[651,980],[651,1007],[635,1017],[617,1008],[612,980],[599,965],[599,942],[609,928],[608,853]]))]
recipe={'schema':'luxel.reference-separation/v1','source_digest':hashlib.sha256(src.read_bytes()).hexdigest(),'canvas':[w,h],'silhouette':silhouette,'regions':[{'id':i,'label':label,'path':path}for i,label,path in regions],
 'coverage':[{'movement':movement,'status':'unsupported_pending_authored_coverage','finding':finding}for movement,finding in [('beak','Source depicts one painted beak and cast shadow; independent lower jaw, mouth interior, concealed face and motion underpaint are missing'),('blink','Face under pupils and authored eyelids missing'),('head','Head/hood concealed surfaces and attachment deformation missing'),('body','Lower body and sleeves are cropped by source boundary')]]}
(HERE/'reference-ornith-recipe.json').write_text(json.dumps(recipe,indent=2)+'\n')
(HERE/'reference-assets.mjs').write_text('export const REFERENCE_ASSET = '+json.dumps({'digest':recipe['source_digest'],'size':[w,h],'original':'data:image/webp;base64,'+base64.b64encode(src.read_bytes()).decode(),'png':'data:image/png;base64,'+base64.b64encode(png.read_bytes()).decode()},separators=(',',':'))+';\nexport const REFERENCE_RECIPE = '+json.dumps(recipe,separators=(',',':'))+';\n')
print('Source preserved; mask command counts:',len(silhouette),[(i,len(p))for i,_,p in regions])
