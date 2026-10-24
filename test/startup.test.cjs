'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const core=require('../app/core.js'),media=require('../app/media.js');
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
function boot(cached,deniedStorage=false,preferences={},sceneList=[]) {
  const nodes=new Map(),frames=[],timers=[],intervals=[],calls=[],events={},documentEvents={};
  let now=0;
  let document;
  class Element {
    constructor(tag='div'){this.tagName=tag.toUpperCase();this.children=[];this.dataset={};this.style={setProperty(k,v){this[k]=v;}};this.attributes={};this.className='';this.classList={contains:x=>this.className.split(' ').includes(x),toggle:(x,on)=>{const list=new Set(this.className.split(' ').filter(Boolean));if(on??!list.has(x))list.add(x);else list.delete(x);this.className=[...list].join(' ');},add:x=>this.classList.toggle(x,true),remove:x=>this.classList.toggle(x,false)};}
    append(...items){this.children.push(...items);}
    replaceChildren(...items){this.children=items;}
    get firstElementChild(){return this.children[0];}
    setAttribute(k,v){this.attributes[k]=v;}
    removeAttribute(k){delete this.attributes[k];}
    getAttribute(k){return this.attributes[k]??null;}
    set src(value){this.attributes.src=value;}
    get src(){return this.attributes.src||'';}
    focus(){document.activeElement=this;this.onfocus?.();}
    blur(){if(document.activeElement===this)document.activeElement=null;}
    querySelectorAll(){return [];}
    querySelector(){return null;}
    addEventListener(){}
    scrollIntoView(){}
    click(){this.onclick?.();}
    pause(){}
    load(){}
    play(){calls.push('video:play');return Promise.resolve();}
  }
  document={body:new Element('body'),documentElement:new Element('html'),activeElement:null,hidden:false,createElement:tag=>new Element(tag),getElementById:id=>{if(!nodes.has(id))nodes.set(id,new Element());return nodes.get(id);},addEventListener(name,fn){(documentEvents[name]||=[]).push(fn);},querySelectorAll(){return [];}};
  const inventory=deferred(),capabilities=deferred(),inputs=deferred();
  const context={document,localStorage:{getItem:()=>JSON.stringify({selectionImported:true,aerial:false,...preferences}),setItem(){}},Date:class extends Date{static now(){return now;}},innerWidth:1920,innerHeight:1080,addEventListener(name,fn){(events[name]||=[]).push(fn);},setInterval(fn,ms){intervals.push({fn,ms});},clearTimeout(timer){if(timer)timer.cancelled=true;},setTimeout(fn,ms=0){const timer=()=>{if(!timer.cancelled){timer.cancelled=true;fn();}};timer.due=now+ms;timers.push(timer);return timer;},requestAnimationFrame:fn=>{frames.push(fn);},performance:{now:()=>now},queueMicrotask,console,
    AerialCore:core,AerialMedia:media,APPLE_AERIALS:{scenes:sceneList},AerialCache:{create:()=>({read:()=>cached,save(){},icon:async()=>null}),iconKey:()=>''},
    TV:{live:true,call:async()=>({volume:20}),listApps:()=>{calls.push('apps');return inventory.promise;},capabilities:()=>{calls.push('caps');return capabilities.promise;},listInputs:()=>{calls.push('inputs');return inputs.promise;},lumaFavorites:async()=>[],launch:async id=>{calls.push('launch:'+id);}}};
  if(deniedStorage)for(const key of ['localStorage','indexedDB'])Object.defineProperty(context,key,{get(){throw Error('Storage denied');}});
  context.window=context;vm.runInNewContext(fs.readFileSync(require.resolve('../app/app.js'),'utf8'),context);
  const fire=(name,detail={},source=events)=>{const e={keyCode:0,preventDefault(){this.prevented=true;},stopImmediatePropagation(){this.stopped=true;},...detail};for(const fn of source[name]||[]){fn(e);if(e.stopped)break;}return e;};
  const key=value=>fire('keydown',{key:value});
  const runTimers=async()=>{for(let i=0;i<100;i++){const next=timers.filter(t=>!t.cancelled&&t.due<=now).sort((a,b)=>a.due-b.due)[0];if(!next)return;next();await new Promise(setImmediate);}throw Error('Timer loop did not settle');};
  return {nodes,frames,timers,calls,inventory,capabilities,inputs,context,document,key,fire,documentEvent:(name,detail)=>fire(name,detail,documentEvents),advance:ms=>{now+=ms;},idleTick:()=>intervals.find(i=>i.ms===5000).fn(),paint:()=>{while(frames.length)frames.shift()();},runTimers};
}
const app={id:'netflix',title:'Netflix',glyph:'play'};

test('clock styles apply on Home and full screen; first wake key cannot launch an app',async()=>{
  const b=boot({apps:[app],caps:null},false,{aerial:false,saverMinutes:0});
  const control=label=>b.nodes.get('panel').children.find(n=>n.tagName==='BUTTON'&&n.children.some(c=>c.textContent===label));
  assert.equal(b.document.body.dataset.clockStyle,'classic');assert.equal(b.nodes.get('hero-clock').hidden,true);
  b.key('c');control('Settings').click();control('Clock style').click();control('Liquid Glass').click();await new Promise(setImmediate);
  assert.equal(b.document.body.dataset.clockStyle,'glass');assert.equal(b.nodes.get('hero-clock').hidden,false);
  assert.equal(control('Liquid Glass').getAttribute('aria-pressed'),'true');assert.equal(b.document.activeElement,control('Liquid Glass'));
  assert.equal(b.nodes.get('home-clock-digits').textContent,b.nodes.get('saver-clock-digits').textContent);
  control('Show date').click();assert.equal(b.nodes.get('home-clock-date').hidden,true);assert.equal(b.nodes.get('saver-clock-date').hidden,true);
  control('Time format').click();control('Time format').click();assert.equal(b.nodes.get('home-clock-period').hidden,true);
  control('Immersive').click();control('View full-screen clock').click();
  assert.equal(b.document.body.dataset.clockStyle,'immersive');assert.equal(b.document.body.classList.contains('saver'),true);assert.equal(b.nodes.get('saver-caption').hidden,false);
  assert.equal(b.calls.includes('video:play'),false);
  b.advance(2*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-faded'),true);
  b.advance(3*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-resting'),true);
  b.key('Enter');b.fire('keyup',{key:'Enter'});assert.equal(b.document.body.classList.contains('saver'),false);assert.equal(b.document.body.classList.contains('oled-resting'),false);
  assert.equal(b.calls.some(c=>c.startsWith('launch:')),false);
  const restored=boot({apps:[app],caps:null},false,{clockStyle:'immersive',clockFormat:'24',clockDate:false});
  assert.equal(restored.document.body.dataset.clockStyle,'immersive');assert.equal(restored.nodes.get('home-clock-period').hidden,true);assert.equal(restored.nodes.get('home-clock-date').hidden,true);
});
test('cached Home is focusable before service requests; refresh preserves the existing tiles',async()=>{
  const b=boot({apps:[app],caps:{dolbyVision:true,hdr10:true,uhd:true}}),grid=b.nodes.get('grid'),tile=grid.firstElementChild;
  assert.equal(tile.dataset.id,'netflix');assert.equal(b.document.activeElement,tile);assert.deepEqual(b.calls,[]);
  // Cached paint is scheduled before starting any TV calls.
  while(b.frames.length)b.frames.shift()();b.timers.shift()();assert.deepEqual(b.calls,['apps','caps']);
  b.inventory.resolve([{...app}]);await new Promise(setImmediate);
  assert.equal(grid.firstElementChild,tile);assert.deepEqual(b.calls,['apps','caps','inputs']);
});
test('fresh inventory renders while capability and HDMI requests remain unresolved',async()=>{
  const b=boot(null);assert.equal(b.nodes.has('grid'),false);
  b.inventory.resolve([{...app}]);await new Promise(setImmediate);
  assert.equal(b.nodes.get('grid').firstElementChild.dataset.id,'netflix');assert.equal(b.context.AerialStartup.capabilitiesMs,undefined);
  assert.deepEqual(b.calls,['apps','caps','inputs']);
});
test('failed discovery keeps cached Home available and reports the error',async()=>{
  const b=boot({apps:[app],caps:null});while(b.frames.length)b.frames.shift()();b.timers.shift()();
  b.inventory.reject(Error('service unavailable'));await new Promise(setImmediate);
  assert.equal(b.nodes.get('grid').firstElementChild.dataset.id,'netflix');assert.match(b.nodes.get('toast').textContent,/service unavailable/);
});
test('denied browser storage getters do not prevent a fresh Home from loading',async()=>{
  const b=boot(null,true);b.inventory.resolve([{...app}]);await new Promise(setImmediate);
  assert.equal(b.nodes.get('grid').firstElementChild.dataset.id,'netflix');
});
test('input row is revealed by Down; focus never launches HDMI; Up hides it and OK selects it',async()=>{
  const input={id:'com.webos.app.hdmi3',title:'PS5 Game Console',input:true};
  const b=boot({apps:[app,input],caps:null});assert.equal(b.nodes.get('grid').children.length,1);
  b.key('ArrowDown');assert.equal(b.nodes.get('input-row').hidden,false);assert.equal(b.document.activeElement.dataset.id,input.id);
  assert.equal(b.calls.some(c=>c.startsWith('launch:')),false);
  b.key('ArrowUp');assert.equal(b.nodes.get('input-row').hidden,true);assert.equal(b.document.activeElement.dataset.id,'netflix');
  b.key('ArrowDown');b.key('Enter');await new Promise(setImmediate);assert.equal(b.calls.includes('launch:'+input.id),true);
});

test('OLED dimming starts at 30 idle minutes even with video disabled, and wake never launches or repeats',async()=>{
  const b=boot({apps:[app],caps:null}),dimmed=()=>b.document.body.classList.contains('oled-dimmed');
  b.advance(30*60000-1);b.idleTick();assert.equal(dimmed(),false);
  b.advance(1);b.idleTick();assert.equal(dimmed(),true);
  assert.equal(b.key('Enter').prevented,true);assert.equal(dimmed(),false);
  assert.equal(b.key('Enter').prevented,true);assert.equal(b.calls.some(c=>c.startsWith('launch:')),false);
  b.fire('keyup',{key:'Enter'});assert.equal(b.calls.some(c=>c.startsWith('launch:')),false);
  b.key('Enter');b.fire('keyup',{key:'Enter'});await new Promise(setImmediate);
  assert.equal(b.calls.includes('launch:netflix'),true);
});

test('OLED dimming respects the off setting and genuine activity restarts its deadline',()=>{
  const off=boot({apps:[app],caps:null},false,{oledDimming:false});off.advance(60*60000);off.idleTick();assert.equal(off.document.body.classList.contains('oled-dimmed'),false);
  const b=boot({apps:[app],caps:null});b.advance(29*60000);b.fire('pointermove');b.advance(2*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-dimmed'),false);
  b.advance(28*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-dimmed'),true);
  assert.equal(b.fire('pointerdown').prevented,true);assert.equal(b.fire('click').prevented,true);assert.equal(b.document.body.classList.contains('oled-dimmed'),false);
  assert.equal(b.fire('pointerdown').prevented,undefined);assert.equal(b.fire('click').prevented,undefined);
});

test('OLED overlay covers open menus and the saver, clears on foregrounding and Home relaunch',()=>{
  const b=boot({apps:[app],caps:null});b.key('c');b.advance(30*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-dimmed'),true);
  const panel=b.nodes.get('panel');b.fire('pointermove');assert.equal(b.nodes.get('panel'),panel);assert.equal(panel.hidden,false);
  b.advance(30*60000);b.document.hidden=true;b.documentEvent('visibilitychange');b.idleTick();assert.equal(b.document.body.classList.contains('oled-dimmed'),false);
  b.document.hidden=false;b.documentEvent('visibilitychange');b.idleTick();assert.equal(b.document.body.classList.contains('oled-dimmed'),false);
  b.advance(30*60000);b.idleTick();b.documentEvent('webOSRelaunch',{detail:{}});assert.equal(b.document.body.classList.contains('oled-dimmed'),false);
  const s=boot({apps:[app],caps:null},false,{aerial:true});s.advance(3*60000);s.idleTick();assert.equal(s.document.body.classList.contains('saver'),true);
  s.advance(27*60000);s.idleTick();assert.equal(s.document.body.classList.contains('oled-dimmed'),true);
  s.key('ArrowDown');assert.equal(s.document.body.classList.contains('saver'),false);assert.equal(s.document.body.classList.contains('oled-dimmed'),false);assert.equal(s.nodes.get('input-row')?.hidden??true,true);
});

test('static protection shifts without resetting on activity; idle fade/hide consume wake input',()=>{
  const b=boot({apps:[app],caps:null},false,{saverMinutes:0,oledDimming:false});
  b.advance(60000);b.idleTick();assert.equal(b.document.documentElement.style['--oled-x'],'4px');
  b.key('ArrowLeft');b.advance(2*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-faded'),true);assert.equal(b.document.documentElement.style['--oled-y'],'4px');
  const focus=b.document.activeElement;b.key('Enter');b.fire('keyup',{key:'Enter'});
  assert.equal(b.document.activeElement,focus);assert.equal(b.calls.some(c=>c.startsWith('launch:')),false);assert.equal(b.document.body.classList.contains('oled-faded'),false);
  b.advance(5*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-resting'),true);
  b.key('ArrowDown');b.fire('keyup',{key:'ArrowDown'});assert.equal(b.document.activeElement,focus);assert.equal(b.nodes.get('input-row')?.hidden??true,true);assert.equal(b.document.body.classList.contains('oled-resting'),false);
  b.key('ArrowDown');assert.equal(b.nodes.get('input-row').hidden,false);
  b.advance(2*60000);b.idleTick();assert.equal(b.fire('wheel').prevented,true);assert.equal(b.document.body.classList.contains('oled-faded'),false);
});

test('static care covers open panels, has persistent previews and can be disabled independently of 30-minute dimming',()=>{
  const b=boot({apps:[app],caps:null});
  const control=label=>b.nodes.get('panel').children.find(n=>n.tagName==='BUTTON'&&n.children.some(c=>c.textContent===label));
  b.key('c');control('Settings').click();control('OLED care').click();
  control('Preview hidden interface').click();b.idleTick();assert.equal(b.document.body.classList.contains('oled-resting'),true);
  b.fire('pointermove');control('Preview idle fade').click();b.idleTick();assert.equal(b.document.body.classList.contains('oled-faded'),true);
  b.fire('pointermove');control('Protect static elements').click();b.advance(5*60000);b.idleTick();
  assert.equal(b.document.body.classList.contains('oled-resting'),false);assert.equal(b.document.body.classList.contains('oled-faded'),false);assert.equal(b.document.documentElement.style['--oled-x'],'0px');
  b.advance(25*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-dimmed'),true);
});

test('hidden app clears static protection and foreground return starts a fresh idle period',()=>{
  const b=boot({apps:[app],caps:null});b.advance(5*60000);b.idleTick();assert.equal(b.document.body.classList.contains('oled-resting'),true);
  assert.equal(b.nodes.get('toast')?.textContent,undefined);
  b.document.hidden=true;b.documentEvent('visibilitychange');assert.equal(b.document.body.classList.contains('oled-resting'),false);assert.equal(b.document.documentElement.style['--oled-x'],'0px');
  b.advance(10*60000);b.document.hidden=false;b.documentEvent('visibilitychange');b.idleTick();assert.equal(b.document.body.classList.contains('oled-resting'),false);assert.equal(b.document.body.classList.contains('oled-faded'),false);
});

const scene={id:'test-aerial',title:'Test aerial',category:'Landscapes',variants:[{url:'https://example.org/aerial-4k.mov',height:2160,fps:29.97,range:'SDR'}]};
const settle=()=>new Promise(setImmediate);
async function readyAerial(preferences={}) {
  const b=boot({apps:[app],caps:null},false,{aerial:true,saverMinutes:0,...preferences},[scene]);
  b.paint();await b.runTimers();b.capabilities.resolve({dolbyVision:false,hdr10:false,uhd:true});await settle();return b;
}
test('first aerial waits for usable Home frames and the configured entry delay before allocating video',async()=>{
  const b=boot(null,false,{aerial:true,saverMinutes:0},[scene]);
  b.capabilities.resolve({dolbyVision:false,hdr10:false,uhd:true});await settle();b.advance(5000);await b.runTimers();assert.equal(b.nodes.get('aerial').src,'');
  b.inventory.resolve([{...app}]);await settle();assert.equal(b.document.activeElement.dataset.id,'netflix');
  b.paint();b.advance(2999);await b.runTimers();assert.equal(b.nodes.get('aerial').src,'');
  b.advance(1);await b.runTimers();assert.equal(b.nodes.get('aerial').src,scene.variants[0].url);assert.equal(b.calls.filter(c=>c==='video:play').length,1);
  assert.equal(b.context.AerialStartup.videoGateReleasedMs,8000);
  const empty=await readyAerial({hidden:['netflix']});assert.equal(empty.nodes.get('grid').children.length,0);
  empty.advance(3000);await empty.runTimers();assert.equal(empty.calls.filter(c=>c==='video:play').length,1);
});
test('navigation postpones initial video until 1.2 seconds quiet, without rebuilding tiles',async()=>{
  const b=await readyAerial(),tile=b.nodes.get('grid').firstElementChild;
  b.advance(2900);b.key('ArrowLeft');b.advance(100);await b.runTimers();assert.equal(b.nodes.get('aerial').src,'');
  b.advance(1099);await b.runTimers();assert.equal(b.nodes.get('aerial').src,'');
  b.advance(1);await b.runTimers();assert.equal(b.calls.filter(c=>c==='video:play').length,1);assert.equal(b.nodes.get('grid').firstElementChild,tile);
});
test('opening a menu postpones pending video, but menus do not pause an aerial already playing',async()=>{
  const b=await readyAerial();b.key('c');b.advance(3000);await b.runTimers();assert.equal(b.nodes.get('aerial').src,'');
  b.key('Escape');b.advance(1200);await b.runTimers();assert.equal(b.calls.filter(c=>c==='video:play').length,1);
  b.key('c');assert.equal(b.nodes.get('aerial').src,scene.variants[0].url);assert.equal(b.calls.filter(c=>c==='video:play').length,1);
});
test('leaving Home cancels delayed playback and returning grants a fresh entry window',async()=>{
  const b=await readyAerial();b.advance(2000);b.document.hidden=true;b.documentEvent('visibilitychange');b.advance(10000);await b.runTimers();assert.equal(b.calls.includes('video:play'),false);
  b.document.hidden=false;b.documentEvent('visibilitychange');b.paint();b.advance(2999);await b.runTimers();assert.equal(b.calls.includes('video:play'),false);
  b.advance(1);await b.runTimers();assert.equal(b.calls.filter(c=>c==='video:play').length,1);
  b.document.hidden=true;b.documentEvent('visibilitychange');assert.equal(b.nodes.get('aerial').src,'');
  b.document.hidden=false;b.documentEvent('visibilitychange');b.paint();b.advance(3000);await b.runTimers();assert.equal(b.calls.filter(c=>c==='video:play').length,2);
});
test('explicit aerial selection and subsequent clips bypass entry delay; zero delay remains available',async()=>{
  const b=await readyAerial();b.documentEvent('webOSRelaunch',{detail:{sceneId:scene.id}});await settle();assert.equal(b.calls.filter(c=>c==='video:play').length,1);
  b.advance(3000);await b.runTimers();assert.equal(b.calls.filter(c=>c==='video:play').length,1);
  b.nodes.get('aerial').onended();await settle();assert.equal(b.calls.filter(c=>c==='video:play').length,2);
  const immediate=await readyAerial({aerialDelay:0});assert.equal(immediate.calls.filter(c=>c==='video:play').length,1);
  const early=boot({apps:[app],caps:null},false,{aerial:true,saverMinutes:0},[scene]);early.paint();await early.runTimers();
  const control=label=>early.nodes.get('panel').children.find(n=>n.tagName==='BUTTON'&&n.children.some(c=>c.textContent===label));
  early.key('c');control('Aerials').click();control('Next aerial').click();await settle();assert.equal(early.calls.filter(c=>c==='video:play').length,1);
  early.capabilities.resolve({dolbyVision:false,hdr10:false,uhd:true});await settle();early.advance(5000);await early.runTimers();assert.equal(early.calls.filter(c=>c==='video:play').length,1);
});
