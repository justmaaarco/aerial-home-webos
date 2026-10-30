'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const core=require('../app/core.js'),media=require('../app/media.js');
function desktop(playlist,options={}){
  const nodes=new Map(),timers=[],events={},intervals=[];let now=0;
  const node=id=>{if(!nodes.has(id))nodes.set(id,{textContent:'',hidden:false,dataset:{},style:{setProperty(k,v){this[k]=v;}},attributes:{},getAttribute(k){return this.attributes[k]??null;},setAttribute(k,v){this.attributes[k]=v;},removeAttribute(k){delete this.attributes[k];},set src(v){this.attributes.src=v;},get src(){return this.attributes.src||'';},play(){this.playing=true;return Promise.resolve();},pause(){this.playing=false;},load(){}});return nodes.get(id);};
  const document={getElementById:node,body:node('body'),documentElement:node('root'),addEventListener(k,fn){events[k]=fn;},hidden:false};
  const context={document,AerialCore:core,AerialMedia:media,AERIAL_DESKTOP_PLAYLIST:playlist,location:{search:''},URLSearchParams,Date:class extends Date{static now(){return now;}},setTimeout(fn){timers.push(fn);return fn;},clearTimeout(){},setInterval(fn){intervals.push(fn);},addEventListener(k,fn){events[k]=fn;}};
  Object.assign(context,options);context.window=context;vm.runInNewContext(fs.readFileSync(require.resolve('../desktop/desktop.js'),'utf8'),context);
  return {context,node,events,advance:ms=>{now+=ms;intervals.forEach(fn=>fn());}};
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
  for(let i=0;i<216;i++){
    const video=d.node('aerial');assert.ok(!seen.has(video.src),'repeated before cycle completion');seen.add(video.src);
    if(i===7)video.onerror();else video.onended();
    if(i===99){const saved=JSON.parse(stored.get('aerial-desktop-cycle-v2'));assert.equal(saved.remaining.length,115);d=desktop(all,{localStorage});}
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
