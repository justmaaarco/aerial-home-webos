'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const {ShuffleBag,variants,label}=require('../app/media.js');
const {clean,homeApps,greeting,starterFavorites}=require('../app/core.js');
test('shuffle plays every entry once per cycle and avoids immediate boundary repeats',()=>{
  const bag=new ShuffleBag(()=>.4),ids=['a','b','c','d'];let last;
  for(let cycle=0;cycle<10;cycle++){const next=ids.map(()=>bag.next(ids));assert.equal(new Set(next).size,4);assert.notEqual(last,next[0]);last=next.at(-1);}
});
test('sequential mode and changing categories remain bounded',()=>{
  const bag=new ShuffleBag(()=>.2);assert.deepEqual([bag.next(['a','b'],false),bag.next(['a','b'],false),bag.next(['a','b'],false)],['a','b','a']);assert.equal(bag.next(['c']), 'c');assert.equal(bag.next([]),null);
});
test('maximum quality respects real Dolby capability and never treats DV5 as HDR10',()=>{
  const scene={variants:[{height:2160,fps:30,range:'HDR',dynamicRange:'Dolby Vision',dolbyProfile:5,url:'dv'},{height:2160,fps:30,range:'SDR',url:'sdr'},{height:2160,fps:240,range:'SDR',url:'mac'}]};
  assert.equal(variants(scene,'auto',{dolbyVision:true,hdr10:true})[0].url,'dv');
  assert.deepEqual(variants(scene,'auto',{dolbyVision:false,hdr10:true}).map(v=>v.url),['sdr']);
  assert.equal(variants(scene,'hdr',{dolbyVision:true,hdr10:true})[0].url,'sdr');
  assert.equal(variants(scene,'sdr',{dolbyVision:true,hdr10:true},'http://localhost:8794').length,2);
  assert.equal(label({height:1080,dynamicRange:'SDR'}),'1080p · SDR');
});
test('home hides utilities and inputs, starts with Luma-style favourites and learns launches',()=>{
  const apps=[{id:'netflix',title:'Netflix'},{id:'youtube',title:'YouTube'},{id:'com.apple.appletv',title:'Apple TV'},{id:'settings',title:'Settings'},{id:'hdmi',title:'HDMI',input:true},{id:'system',title:'Apps'},{id:'plex',title:'Plex'}];
  let p=clean();assert.deepEqual(homeApps(apps,p).map(a=>a.id),['netflix','youtube','com.apple.appletv']);
  p=clean({usage:{plex:{count:4,last:10}},hidden:['youtube']});assert.deepEqual(homeApps(apps,p).map(a=>a.id),['plex','netflix','com.apple.appletv']);
  assert.equal(starterFavorites([{id:'demo-tv',title:'TV'}])[0],'demo-tv');
});
test('greetings respect time boundaries and rotate phrases without immediate repeats',()=>{
  for(const [hour,period] of [[4,'night'],[5,'morning'],[11,'morning'],[12,'afternoon'],[16,'afternoon'],[17,'evening'],[21,'evening'],[22,'night']])assert.equal(greeting(hour,()=>0).period,period);
  assert.notEqual(greeting(10,()=>0,0).phrase,greeting(10,()=>0).phrase);
});
test('catalogue preserves all eight Apple sources, real ranges and 4K variants',()=>{
  const scope={window:{}};vm.runInNewContext(fs.readFileSync(require.resolve('../app/catalogue.js'),'utf8'),scope);const catalog=scope.window.APPLE_AERIALS;
  assert.equal(catalog.sources.length,8);assert.ok(catalog.scenes.length>=150);
  assert.equal(catalog.sources.find(s=>s.name==='macos26').entries,152);
  assert.equal(catalog.sources.find(s=>s.name==='tvos16').entries,114);
  assert.equal(new Set(catalog.scenes.map(s=>s.id)).size,catalog.scenes.length);
  const dolby=new Set();for(const scene of catalog.scenes)for(const v of scene.variants){assert.ok(['sylvan.apple.com','a1.phobos.apple.com'].includes(new URL(v.url).hostname));if(v.dynamicRange==='Dolby Vision'){dolby.add(v.url);assert.equal(v.height,2160);assert.equal(v.dolbyProfile,5);}}
  assert.equal(dolby.size,114);
});
