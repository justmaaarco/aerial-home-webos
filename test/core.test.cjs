'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict');
const {clean,visible,nextIndex}=require('../app/core.js');
test('OLED dimming defaults on for existing preferences and accepts only saved booleans',()=>{
  assert.equal(clean().oledDimming,true);assert.equal(clean({oledDimming:false}).oledDimming,false);assert.equal(clean({oledDimming:'false'}).oledDimming,true);
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
