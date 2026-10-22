'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const core=require('../app/core.js'),media=require('../app/media.js');
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
function boot(cached,deniedStorage=false,preferences={}) {
  const nodes=new Map(),frames=[],timers=[],intervals=[],calls=[],events={},documentEvents={};
  let now=0;
  let document;
  class Element {
    constructor(tag='div'){this.tagName=tag.toUpperCase();this.children=[];this.dataset={};this.style={setProperty(){}};this.attributes={};this.className='';this.classList={contains:x=>this.className.split(' ').includes(x),toggle:(x,on)=>{const list=new Set(this.className.split(' ').filter(Boolean));if(on??!list.has(x))list.add(x);else list.delete(x);this.className=[...list].join(' ');},add:x=>this.classList.toggle(x,true),remove:x=>this.classList.toggle(x,false)};}
    append(...items){this.children.push(...items);}
    replaceChildren(...items){this.children=items;}
    get firstElementChild(){return this.children[0];}
    setAttribute(k,v){this.attributes[k]=v;}
    removeAttribute(k){delete this.attributes[k];}
    getAttribute(k){return this.attributes[k]??null;}
    focus(){document.activeElement=this;this.onfocus?.();}
    blur(){if(document.activeElement===this)document.activeElement=null;}
    querySelectorAll(){return [];}
    querySelector(){return null;}
    addEventListener(){}
    scrollIntoView(){}
    click(){this.onclick?.();}
    pause(){}
    load(){}
  }
  document={body:new Element('body'),documentElement:new Element('html'),activeElement:null,hidden:false,createElement:tag=>new Element(tag),getElementById:id=>{if(!nodes.has(id))nodes.set(id,new Element());return nodes.get(id);},addEventListener(name,fn){(documentEvents[name]||=[]).push(fn);},querySelectorAll(){return [];}};
  const inventory=deferred(),capabilities=deferred(),inputs=deferred();
  const context={document,localStorage:{getItem:()=>JSON.stringify({selectionImported:true,aerial:false,...preferences}),setItem(){}},Date:class extends Date{static now(){return now;}},innerWidth:1920,innerHeight:1080,addEventListener(name,fn){(events[name]||=[]).push(fn);},setInterval(fn,ms){intervals.push({fn,ms});},clearTimeout(){},setTimeout:fn=>{timers.push(fn);},requestAnimationFrame:fn=>{frames.push(fn);},performance:{now:()=>0},queueMicrotask,console,
    AerialCore:core,AerialMedia:media,APPLE_AERIALS:{scenes:[]},AerialCache:{create:()=>({read:()=>cached,save(){},icon:async()=>null}),iconKey:()=>''},
    TV:{live:true,call:async()=>({volume:20}),listApps:()=>{calls.push('apps');return inventory.promise;},capabilities:()=>{calls.push('caps');return capabilities.promise;},listInputs:()=>{calls.push('inputs');return inputs.promise;},lumaFavorites:async()=>[],launch:async id=>{calls.push('launch:'+id);}}};
  if(deniedStorage)for(const key of ['localStorage','indexedDB'])Object.defineProperty(context,key,{get(){throw Error('Storage denied');}});
  context.window=context;vm.runInNewContext(fs.readFileSync(require.resolve('../app/app.js'),'utf8'),context);
  const fire=(name,detail={},source=events)=>{const e={keyCode:0,preventDefault(){this.prevented=true;},stopImmediatePropagation(){this.stopped=true;},...detail};for(const fn of source[name]||[]){fn(e);if(e.stopped)break;}return e;};
  const key=value=>fire('keydown',{key:value});
  return {nodes,frames,timers,calls,inventory,capabilities,inputs,context,document,key,fire,documentEvent:(name,detail)=>fire(name,detail,documentEvents),advance:ms=>{now+=ms;},idleTick:()=>intervals.find(i=>i.ms===5000).fn()};
}
const app={id:'netflix',title:'Netflix',glyph:'play'};
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
