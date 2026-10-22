"""Rebuild the Apple URL catalogue from Apple's published manifests (no media download)."""
import argparse, collections, hashlib, io, json, pathlib, plistlib, re, tarfile, urllib.request

BASE = 'http://sylvan.apple.com/itunes-assets/Aerials126/v4/82/2e/34/822e344c-f5d2-878c-3d56-508d5b09ed61/'
SOURCES = [
    ('tvos10', 'http://a1.phobos.apple.com/us/r1000/000/Features/atv/AutumnResources/videos/entries.json', 'tvos10.json'),
    ('tvos11', 'http://sylvan.apple.com/Aerials/2x/entries.json', 'tvos11.json'),
    ('tvos12', 'http://sylvan.apple.com/Aerials/resources.tar', 'resources.tar'),
    ('tvos13', 'http://sylvan.apple.com/Aerials/resources-13.tar', 'tvos13.tar'),
    ('tvos16', 'http://sylvan.apple.com/Aerials/resources-16.tar', 'tvos16.tar'),
    ('macos14', BASE+'resources-14-0-10.tar', 'macos14.tar'),
    ('macos15', BASE+'resources-15-0-2.tar', 'macos15.tar'),
    ('macos26', BASE+'resources-26-0-1.tar', 'macos26.tar'),
]

def category_name(key):
    return re.sub(r'(?<!^)(?=[A-Z])', ' ', key.replace('AerialCategory', '')).strip() or 'Other'

def build(cache, probe_path=None):
    cache.mkdir(parents=True, exist_ok=True)
    records, provenance, aliases = {}, [], {}
    for source, url, filename in SOURCES:
        file = cache/filename
        if not file.exists():
            with urllib.request.urlopen(url, timeout=30) as response:
                file.write_bytes(response.read(20_000_000))
        raw=file.read_bytes(); strings={}
        if filename.endswith('.tar'):
            with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
                entry=next(m for m in archive.getmembers() if m.name.lstrip('./')=='entries.json')
                manifest=json.load(archive.extractfile(entry))
                for member in archive.getmembers():
                    if '/en.lproj/' in member.name and member.name.endswith('.strings'):
                        try: strings.update(plistlib.loads(archive.extractfile(member).read()))
                        except Exception: pass
        else: manifest=json.loads(raw)
        entries=manifest.get('assets',[]) if isinstance(manifest,dict) else [asset for group in manifest for asset in group.get('assets',[])]
        categories={c['id']:category_name(c.get('localizedNameKey','Other')) for c in manifest.get('categories',[])} if isinstance(manifest,dict) else {}
        for a in entries:
            # Legacy groups also contain static image records; only import actual video URLs.
            variants=[]
            for key,value in a.items():
                if (key.startswith('url-') or key=='url') and isinstance(value,str) and value.endswith(('.mov','.mp4','.m4v')):
                    parsed=urllib.parse.urlparse(value)
                    if parsed.hostname not in ('sylvan.apple.com','a1.phobos.apple.com'):continue
                    variants.append({'key':key,'url':value.replace('https://','http://',1),'height':2160 if '4K' in key else 1080,'range':'HDR' if 'HDR' in key else 'SDR','fps':240 if '240FPS' in key else 30,'source':source})
            if not variants:continue
            aid=a.get('id') or hashlib.sha256(variants[0]['url'].encode()).hexdigest()[:24]
            shot=a.get('shotID')
            rid=aliases.get(aid) or (aliases.get(shot) if shot else None) or next((aliases[v['url']] for v in variants if v['url'] in aliases),None) or aid
            aliases[aid]=rid
            if shot:aliases[shot]=rid
            for variant in variants:aliases[variant['url']]=rid
            rec=records.setdefault(rid,{'id':rid,'title':a.get('accessibilityLabel') or a.get('name') or 'Apple Aerial','category':'Other','variants':[],'sources':[]})
            title=strings.get(a.get('localizedNameKey')) or a.get('accessibilityLabel') or a.get('name')
            if title:rec['title']=title
            cat=next((categories[c] for c in a.get('categories',[]) if c in categories),None)
            if cat:rec['category']=cat
            if source not in rec['sources']:rec['sources'].append(source)
            known={v['url'] for v in rec['variants']}
            for variant in variants:
                if variant['url'] not in known:rec['variants'].append(variant);known.add(variant['url'])
        provenance.append({'name':source,'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'entries':len(entries)})
    # A small optional probe index records actual codec/range/Dolby metadata, never inferred badges.
    probes=json.loads(pathlib.Path(probe_path).read_text()) if probe_path else {}
    probes=probes.get('probes',probes)
    scenes=[]
    for rec in records.values():
        rec['variants'].sort(key=lambda v:(-v['height'],v['fps']>60,v['range']!='HDR',-next((i for i,s in enumerate(SOURCES) if s[0]==v['source']),0),v['key']))
        for v in rec['variants']:
            if v['url'] in probes:v.update(probes[v['url']])
        if rec['category']=='Other':
            if re.search('space|earth|orbit|night',rec['title'],re.I):rec['category']='Space'
            else:rec['category']='Cities'
        scenes.append(rec)
    scenes.sort(key=lambda s:(s['category'],s['title'],s['id']))
    return {'schema':1,'sources':provenance,'scenes':scenes}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--cache-dir',required=True);parser.add_argument('--probes',default='tools/apple-format-probes.json');parser.add_argument('--output',default='app/catalogue.js');args=parser.parse_args()
    catalog=build(pathlib.Path(args.cache_dir),args.probes)
    pathlib.Path(args.output).write_text('/* Apple media URL metadata; no video files. Generated by tools/update-catalogue.py. */\nwindow.APPLE_AERIALS='+json.dumps(catalog,ensure_ascii=False,separators=(',',':'))+';\n',encoding='utf-8',newline='\n')
    print(json.dumps({'scenes':len(catalog['scenes']),'variants':sum(len(s['variants']) for s in catalog['scenes']),'categories':dict(collections.Counter(s['category'] for s in catalog['scenes']))}))
