'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const core=require('../app/core.js'),media=require('../app/media.js');
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
function boot(cached,deniedStorage=false) {
  const nodes=new Map(),frames=[],timers=[],calls=[],events={};
  let document;
  class Element {
    constructor(tag='div'){this.tagName=tag.toUpperCase();this.children=[];this.dataset={};this.style={setProperty(){}};this.attributes={};this.className='';this.classList={contains:x=>this.className.split(' ').includes(x),toggle(){},add(){},remove(){}};}
    append(...items){this.children.push(...items);}
    replaceChildren(...items){this.children=items;}
    get firstElementChild(){return this.children[0];}
    setAttribute(k,v){this.attributes[k]=v;}
    removeAttribute(k){delete this.attributes[k];}
    focus(){document.activeElement=this;this.onfocus?.();}
    querySelectorAll(){return [];}
    querySelector(){return null;}
    addEventListener(){}
    scrollIntoView(){}
    click(){this.onclick?.();}
    pause(){}
    load(){}
  }
  document={body:new Element('body'),documentElement:new Element('html'),activeElement:null,hidden:false,createElement:tag=>new Element(tag),getElementById:id=>{if(!nodes.has(id))nodes.set(id,new Element());return nodes.get(id);},addEventListener(){},querySelectorAll(){return [];}};
  const inventory=deferred(),capabilities=deferred(),inputs=deferred();
  const context={document,localStorage:{getItem:()=>JSON.stringify({selectionImported:true,aerial:false}),setItem(){}},innerWidth:1920,innerHeight:1080,addEventListener(name,fn){(events[name]||=[]).push(fn);},setInterval(){},clearTimeout(){},setTimeout:fn=>{timers.push(fn);},requestAnimationFrame:fn=>{frames.push(fn);},performance:{now:()=>0},queueMicrotask,console,
    AerialCore:core,AerialMedia:media,APPLE_AERIALS:{scenes:[]},AerialCache:{create:()=>({read:()=>cached,save(){},icon:async()=>null}),iconKey:()=>''},
    TV:{live:true,listApps:()=>{calls.push('apps');return inventory.promise;},capabilities:()=>{calls.push('caps');return capabilities.promise;},listInputs:()=>{calls.push('inputs');return inputs.promise;},lumaFavorites:async()=>[],launch:async id=>{calls.push('launch:'+id);}}};
  if(deniedStorage)for(const key of ['localStorage','indexedDB'])Object.defineProperty(context,key,{get(){throw Error('Storage denied');}});
  context.window=context;vm.runInNewContext(fs.readFileSync(require.resolve('../app/app.js'),'utf8'),context);
  const key=value=>events.keydown.forEach(fn=>fn({key:value,keyCode:0,preventDefault(){}}));
  return {nodes,frames,timers,calls,inventory,capabilities,inputs,context,document,key};
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
