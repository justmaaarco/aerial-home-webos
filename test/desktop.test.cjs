'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const core=require('../app/core.js'),media=require('../app/media.js');
function desktop(playlist,options={}){
  const nodes=new Map(),timers=[],events={},intervals=[];let now=0;
  const node=id=>{if(!nodes.has(id))nodes.set(id,{textContent:'',hidden:false,dataset:{},style:{setProperty(k,v){this[k]=v;}},attributes:{},getAttribute(k){return this.attributes[k]??null;},setAttribute(k,v){this.attributes[k]=v;},removeAttribute(k){delete this.attributes[k];},set src(v){this.attributes.src=v;},get src(){return this.attributes.src||'';},play(){this.playing=true;return Promise.resolve();},pause(){this.playing=false;},load(){}});return nodes.get(id);};
  const document={getElementById:node,body:node('body'),documentElement:node('root'),addEventListener(k,fn){events[k]=fn;},hidden:false};
  const context={document,AerialCore:core,AerialMedia:media,AERIAL_DESKTOP_PLAYLIST:playlist,location:{search:''},URLSearchParams,AbortController,Date:class extends Date{static now(){return now;}},setTimeout(fn,delay){timers.push({fn,delay});return fn;},clearTimeout(fn){const entry=timers.find(t=>t.fn===fn);if(entry)entry.cancelled=true;},setInterval(fn){intervals.push(fn);},addEventListener(k,fn){events[k]=fn;}};
  Object.assign(context,options);context.window=context;vm.runInNewContext(fs.readFileSync(require.resolve('../desktop/desktop.js'),'utf8'),context);
  return {context,node,events,advance:ms=>{now+=ms;intervals.forEach(fn=>fn());},fire:delay=>{for(const t of timers.filter(t=>!t.cancelled&&t.delay===delay)){t.cancelled=true;t.fn();}}};
}
const scenes=[{id:'one',title:'First',file:'videos/one.webm'},{id:'two',title:'Second',file:'videos/two.webm'}];
test('desktop host properties validate clocks independently, pause playback and retain OLED care',()=>{
  const d=desktop(scenes),host=d.context.wallpaperPropertyListener,video=d.node('aerial');
  assert.equal(d.node('body').dataset.style,'glass');assert.equal(video.muted,true);
  host.applyUserProperties({clockstyle:{value:'immersive'},clockformat:{value:'24'},showdate:{value:false},clockposition:{value:'center'}});
  assert.equal(d.node('body').dataset.style,'immersive');assert.equal(d.node('body').dataset.position,'center');assert.equal(d.node('date').hidden,true);assert.equal(d.node('period').hidden,true);
  host.setPaused(true);assert.equal(video.playing,false);host.setPaused(false);assert.equal(video.playing,true);
  d.advance(2*60000);assert.equal(d.node('body').dataset.rest,'faded');d.advance(3*60000);assert.equal(d.node('body').dataset.rest,'hidden');
  d.events.mousemove();assert.equal(d.node('body').dataset.rest,'active');
  host.applyUserProperties({clockstyle:{value:'<script>'},clockposition:{value:'../outside'}});
  assert.equal(d.node('body').dataset.style,'classic');assert.equal(d.node('body').dataset.position,'top');
});
test('desktop only loads packaged video paths, skips failures and stops after all local clips fail',()=>{
  const d=desktop([...scenes,{id:'evil',title:'External',file:'https://example.org/video.webm'}]),video=d.node('aerial'),first=video.src;
  assert.match(first,/^videos\/(one|two)\.webm$/);video.onerror();assert.notEqual(video.src,first);video.onerror();assert.equal(video.src,'');
  assert.match(d.node('status').textContent,/No compatible local aerial/);
  const empty=desktop([{id:'outside',file:'../outside.webm'}]);assert.equal(empty.node('aerial').src,'');assert.match(empty.node('status').textContent,/Add local aerials/);
});
test('desktop full-clip transitions shuffle without immediate repeats and support sequential mode',()=>{
  const d=desktop(scenes),video=d.node('aerial'),first=video.src;video.onended();assert.notEqual(video.src,first);
  d.context.wallpaperPropertyListener.applyUserProperties({shuffle:{value:false}});
  video.onended();const sequential=video.src;video.onended();assert.notEqual(video.src,sequential);video.onended();assert.equal(video.src,sequential);
});
test('a complete catalogue cycle survives a failed URL and reload without repeating watched entries',()=>{
  const all=Array.from({length:216},(_,i)=>({id:'clip-'+i,title:'Clip '+i,file:'videos/clip-'+i+'.webm'})),stored=new Map(),localStorage={getItem:k=>stored.get(k)||null,setItem:(k,v)=>stored.set(k,v)};
  let d=desktop(all,{localStorage}),seen=new Set();
  for(let i=0;seen.size<216;i++){
    assert.ok(i<216,'cycle did not complete');
    const video=d.node('aerial');assert.ok(!seen.has(video.src),'repeated before cycle completion');seen.add(video.src);
    if(seen.size===216)break;
    if(i===7)video.onerror();else video.onended();
    if(i===99){const saved=JSON.parse(stored.get('aerial-desktop-cycle-v2'));assert.equal(saved.remaining.length,115);assert.ok(!seen.has(video.src));seen.add(video.src);d=desktop(all,{localStorage});assert.notEqual(d.node('aerial').src,video.src);assert.ok(!seen.has(d.node('aerial').src));}
  }
  assert.equal(seen.size,216);
});
test('catalogue accepts only its fixed loopback ID endpoint and cancels GPU streaming on pause',async()=>{
  const localStorage={getItem:()=>null,setItem:()=>{}},item={id:'safe',title:'Safe',streaming:true,file:'http://127.0.0.1:5194/video/safe.webm'};
  const d=desktop([item,{...item,id:'bad',file:'http://127.0.0.1:5194/video/other.webm'}],{localStorage,AERIAL_DESKTOP_CATALOGUE_MODE:true,fetch:async()=>({ok:true,json:async()=>({cached:false})})});
  await new Promise(resolve=>setImmediate(resolve));assert.equal(d.node('aerial').src,item.file);
  d.node('aerial').currentTime=23;d.node('aerial').onplaying();d.context.wallpaperPropertyListener.setPaused(true);assert.equal(d.node('aerial').src,'');
  d.context.wallpaperPropertyListener.setPaused(false);await new Promise(resolve=>setImmediate(resolve));assert.equal(d.node('aerial').src,item.file+'?t=23');
});
test('next control skips once on each toggle, never on initial properties or unchanged values',()=>{
  const all=Array.from({length:5},(_,i)=>({id:'next-'+i,title:'Next '+i,file:'videos/next-'+i+'.webm'}));
  const d=desktop(all),host=d.context.wallpaperPropertyListener,video=d.node('aerial'),first=video.src;
  host.applyUserProperties({nextaerial:{value:true},shuffle:{value:true},oledcare:{value:true}});assert.equal(video.src,first);
  host.applyUserProperties({nextaerial:{value:false}});const second=video.src;assert.notEqual(second,first);
  host.applyUserProperties({nextaerial:{value:false},oledcare:{value:false}});assert.equal(video.src,second);
  host.setPaused(true);host.applyUserProperties({nextaerial:{value:true}});assert.equal(video.src,'');
  host.setPaused(false);assert.notEqual(video.src,second);assert.equal(video.playing,true);
});
test('catalogue startup samples the full shuffle rather than preferring a local seed',()=>{
  const all=[{id:'seed',title:'Seed',local:true,file:'videos/seed.webm'},{id:'other',title:'Other',file:'videos/other.webm'}];
  // Fisher-Yates with 0 puts the second item first; an explicit seed preference would override it.
  class FixedBag extends media.ShuffleBag{constructor(){super(()=>0);}}
  const d=desktop(all,{AERIAL_DESKTOP_CATALOGUE_MODE:true,AerialMedia:{...media,ShuffleBag:FixedBag}});
  assert.equal(d.node('aerial').src,'videos/other.webm');
});
test('skipping an online aerial detaches its encoder stream before querying the next source',async()=>{
  const all=['stream-one','stream-two'].map(id=>({id,title:id,streaming:true,file:'http://127.0.0.1:5194/video/'+id+'.webm'}));
  let d;const observed=[];
  d=desktop(all,{AERIAL_DESKTOP_CATALOGUE_MODE:true,fetch:async url=>{if(url.includes('/api/info/'))observed.push(d?.node('aerial').src||'');return {ok:true,json:async()=>({cached:false,busy:false})};}});
  await new Promise(resolve=>setImmediate(resolve));const video=d.node('aerial'),first=video.src;
  d.context.wallpaperPropertyListener.applyUserProperties({nextaerial:{value:false}});
  d.context.wallpaperPropertyListener.applyUserProperties({nextaerial:{value:true}});
  assert.equal(video.src,'');await new Promise(resolve=>setImmediate(resolve));
  assert.notEqual(video.src,first);assert.deepEqual(observed,['','']);
});
test('the first Next click works when initial host properties omit Next or no initial batch arrives',()=>{
  for(const initial of [null,{shuffle:{value:true},oledcare:{value:true}}]){
    const d=desktop(scenes),host=d.context.wallpaperPropertyListener,first=d.node('aerial').src;
    if(initial)host.applyUserProperties(initial);
    host.applyUserProperties({nextaerial:{value:true}});assert.notEqual(d.node('aerial').src,first);
  }
});
test('returning from a game reloads the decoder at the saved position and restores its loading timeout',()=>{
  const d=desktop(scenes),host=d.context.wallpaperPropertyListener,video=d.node('aerial'),first=video.src;
  video.currentTime=42;video.onplaying();host.setPaused(true);assert.equal(video.src,'');
  d.advance(3*3600000);host.setPaused(false);assert.equal(video.src,first);
  video.duration=100;video.onloadedmetadata();assert.equal(video.currentTime,42);
  // A decoder that never reaches onplaying must not wait forever after resume.
  d.fire(20000);assert.notEqual(video.src,first);
});
test('host pause and document visibility independently keep playback stopped',()=>{
  const d=desktop(scenes),host=d.context.wallpaperPropertyListener,video=d.node('aerial');video.onplaying();
  host.setPaused(true);d.context.document.hidden=true;d.events.visibilitychange();
  d.context.document.hidden=false;d.events.visibilitychange();assert.equal(video.src,'');
  host.setPaused(false);assert.ok(video.src);
  d.context.document.hidden=true;d.events.visibilitychange();host.setPaused(false);assert.equal(video.src,'');
  d.context.document.hidden=false;d.events.visibilitychange();assert.ok(video.src);
});
test('a frozen playing decoder is retried once, then skipped if it still makes no progress',()=>{
  const d=desktop(scenes),video=d.node('aerial'),first=video.src;
  video.currentTime=12;video.onplaying();d.advance(20000);
  assert.equal(video.src,first);video.onloadedmetadata();assert.equal(video.currentTime,12);video.onplaying();
  d.advance(20000);assert.notEqual(video.src,first);
});
test('a busy encoder uses a ready aerial while preserving the deferred shuffled scene',async()=>{
  const all=['online','ready'].map(id=>({id,title:id,streaming:true,file:'http://127.0.0.1:5194/video/'+id+'.webm'}));
  class FixedBag extends media.ShuffleBag{constructor(){super(()=>0.99);}}
  const stored=new Map(),d=desktop(all,{AERIAL_DESKTOP_CATALOGUE_MODE:true,AerialMedia:{...media,ShuffleBag:FixedBag},localStorage:{getItem:k=>stored.get(k)||null,setItem:(k,v)=>stored.set(k,v)},fetch:async url=>({ok:true,json:async()=>({cached:url.endsWith('/ready'),busy:true,ready:['ready','arbitrary-url']})})});
  await new Promise(resolve=>setImmediate(resolve));assert.equal(d.node('aerial').src,all[1].file);
  assert.deepEqual(JSON.parse(stored.get('aerial-desktop-cycle-v2')).remaining,['online']);
});
test('hung info requests time out to a local fallback and are cancelled when gaming pauses playback',async()=>{
  const all=[{id:'online',title:'Online',streaming:true,file:'http://127.0.0.1:5194/video/online.webm'},{id:'local',title:'Local',local:true,file:'videos/local.webm'}];
  class FixedBag extends media.ShuffleBag{constructor(){super(()=>0.99);}}
  let signal;
  const d=desktop(all,{AERIAL_DESKTOP_CATALOGUE_MODE:true,AerialMedia:{...media,ShuffleBag:FixedBag},fetch:(url,options)=>url.includes('/api/info/')?new Promise((_,reject)=>{signal=options.signal;signal.addEventListener('abort',()=>reject(Error('aborted')));}):Promise.resolve({ok:true})});
  d.fire(8000);await new Promise(resolve=>setImmediate(resolve));assert.equal(d.node('aerial').src,'videos/local.webm');
  d.node('aerial').onended();assert.equal(signal.aborted,false);
  d.context.wallpaperPropertyListener.setPaused(true);assert.equal(signal.aborted,true);
  await new Promise(resolve=>setImmediate(resolve));assert.equal(d.node('aerial').src,'');
});
