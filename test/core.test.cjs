'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict');
const {clean,visible,nextIndex,oledState,clockFace}=require('../app/core.js');
test('clock preferences migrate to Classic and reject unsupported styles or display options',()=>{
  const old=clean({profile:'Home'});assert.equal(old.clockStyle,'classic');assert.equal(old.clockFormat,'auto');assert.equal(old.clockDate,true);
  for(const clockStyle of ['classic','minimal','glass','immersive'])assert.equal(clean({clockStyle}).clockStyle,clockStyle);
  const invalid=clean({clockStyle:'<style>',clockFormat:24,clockDate:'false'});
  assert.equal(invalid.clockStyle,'classic');assert.equal(invalid.clockFormat,'auto');assert.equal(invalid.clockDate,true);
  assert.equal(clean({clockDate:false}).clockDate,false);
});
test('clock format respects locale, explicit 12/24-hour choice and midnight without seconds',()=>{
  const midnight=new Date(2026,9,24,0,5,42),noon=new Date(2026,9,24,12,5,59);
  const gb=clockFace(midnight,clean(),'en-GB'),us=clockFace(midnight,clean(),'en-US');
  assert.equal(gb.time,'0:05');assert.equal(gb.period,'');assert.equal(us.time,'12:05');assert.equal(us.period,'AM');
  const twentyFour=clockFace(midnight,clean({clockFormat:'24'}),'en-US');assert.equal(twentyFour.time,'00:05');assert.equal(twentyFour.period,'');
  const twelve=clockFace(noon,clean({clockFormat:'12'}),'en-US');assert.equal(twelve.time,'12:05');assert.equal(twelve.period,'PM');assert.match(twelve.date,/Saturday.*October 24/);
  assert.equal(clockFace(noon,clean({clockDate:false}),'en-GB').date,'');
  assert.equal(clockFace(new Date(2026,9,24,0,5,0),clean(),'en-GB').time,gb.time);
});
test('aerial entry delay defaults to three seconds and preserves only supported timings',()=>{
  assert.equal(clean().aerialDelay,3);for(const value of [0,3,5,10])assert.equal(clean({aerialDelay:value}).aerialDelay,value);
  for(const value of [-1,2,NaN,'5',null])assert.equal(clean({aerialDelay:value}).aerialDelay,3);
});
test('OLED care defaults on for existing preferences and accepts only saved booleans',()=>{
  assert.equal(clean().oledDimming,true);assert.equal(clean({oledDimming:false}).oledDimming,false);assert.equal(clean({oledDimming:'false'}).oledDimming,true);
  assert.equal(clean().oledProtection,true);assert.equal(clean({oledProtection:false}).oledProtection,false);assert.equal(clean({oledProtection:'false'}).oledProtection,true);
});
test('OLED shifts stay bounded, visit nine positions and respect off, visibility and Reduce motion',()=>{
  const prefs=clean(),positions=Array.from({length:9},(_,i)=>oledState(prefs,0,i*60000));
  assert.equal(new Set(positions.map(p=>p.x+','+p.y)).size,9);
  for(const p of positions){assert.ok(Math.abs(p.x)<=4);assert.ok(Math.abs(p.y)<=4);}
  assert.deepEqual(oledState(prefs,0,59999),positions[0]);assert.deepEqual(oledState(prefs,0,9*60000),positions[0]);
  assert.deepEqual(oledState({...prefs,oledProtection:false},5*60000,60000),{x:0,y:0,rest:'active'});
  assert.deepEqual(oledState(prefs,5*60000,60000,true),{x:0,y:0,rest:'active'});
  assert.deepEqual(oledState({...prefs,reduceMotion:true},2*60000,60000),{x:0,y:0,rest:'faded'});
});
test('static UI fades at two minutes and disappears at five, independently of video and full dimming',()=>{
  const prefs=clean({aerial:false,saverMinutes:0,oledDimming:false});
  assert.equal(oledState(prefs,2*60000-1,0).rest,'active');assert.equal(oledState(prefs,2*60000,0).rest,'faded');
  assert.equal(oledState(prefs,5*60000-1,0).rest,'faded');assert.equal(oledState(prefs,5*60000,0).rest,'hidden');
});
test('corrupt settings cannot break boot or inject folder records',()=>{
  const p=clean({columns:70,scene:-4,saverMinutes:0,order:['a','a',null],folders:[null,{id:'../../bad',title:'bad',apps:[]},{id:'folder-ok',title:'Films',apps:['a','a',null,'folder-other']}],profile:'x'.repeat(80)});
  assert.equal(p.columns,6);assert.equal(p.scene,2);assert.equal(p.saverMinutes,0);assert.deepEqual(p.order,['a']);assert.equal(p.profile.length,32);assert.deepEqual(p.folders,[{id:'folder-ok',title:'Films',apps:['a']}]);
});
test('folders, ordering, search and hidden apps compose without duplicating apps',()=>{
  const apps=[{id:'a',title:'Cinema'},{id:'b',title:'Games'},{id:'c',title:'Music'}];
  const p=clean({order:['folder-one','c'],hidden:['b'],folders:[{id:'folder-one',title:'Movies',apps:['a','gone']}]});
  assert.deepEqual(visible(apps,p).map(a=>a.id),['folder-one','c']);
  assert.deepEqual(visible(apps,p,'cin').map(a=>a.id),['a']);
  assert.deepEqual(visible(apps,p,'','folder-one').map(a=>a.id),['a']);
  assert.equal(visible(apps,p,'Games').length,0);
});
test('direction navigation reaches partial last rows without wrapping at edges',()=>{
  assert.equal(nextIndex(0,'up',14,6),-1);assert.equal(nextIndex(5,'right',14,6),5);assert.equal(nextIndex(6,'left',14,6),6);assert.equal(nextIndex(11,'down',14,6),13);assert.equal(nextIndex(12,'down',14,6),13);assert.equal(nextIndex(10,'up',14,5),5);assert.equal(nextIndex(0,'down',0,6),-1);
});
