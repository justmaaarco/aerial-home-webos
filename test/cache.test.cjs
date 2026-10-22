'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict');
const {create,snapshot,validIcon,iconKey}=require('../app/cache.js');
test('startup cache rejects corrupt/expired data and strips images and unknown fields from inventory',()=>{
  const now=Date.now(),app={id:'netflix',title:'Netflix',icon:'icon.png',iconBase:'/usr/palm/applications/netflix',image:'large artwork',secret:'not metadata'};
  assert.equal(snapshot({version:0,savedAt:now,apps:[app]}),null);
  assert.equal(snapshot({version:1,savedAt:now-8*86400000,apps:[app]}),null);
  assert.equal(snapshot({version:1,savedAt:now+86400000,apps:[app]}),null);
  const data=snapshot({version:1,savedAt:now,apps:[null,app,app,{id:'bad;id',title:'unsafe'}],caps:{dolbyVision:true,hdr10:true,uhd:true}});
  assert.equal(data.apps.length,1);assert.equal(data.apps[0].image,undefined);assert.equal(data.apps[0].secret,undefined);assert.equal(data.caps.dolbyVision,true);
});
test('cache is optional when storage is denied, full, corrupted or IndexedDB is unavailable',async()=>{
  const blocked=create({getItem(){throw Error('denied');},setItem(){throw Error('full');}},null);
  assert.equal(blocked.read(),null);blocked.save([{id:'a',title:'A'}],null);assert.equal(await blocked.icon({id:'a'}),null);
  let raw='{broken';const cache=create({getItem:()=>raw,setItem:(_,v)=>raw=v},null);
  assert.equal(cache.read(),null);cache.save([{id:'a',title:'A',image:'private artwork'}],{dolbyVision:false,hdr10:false,uhd:true});
  assert.equal(cache.read().apps[0].id,'a');assert.equal(raw.includes('private artwork'),false);
});
test('artwork is reusable only for the same asset source and a bounded image payload',()=>{
  const source=iconKey({icon:'icon.png',iconBase:'/usr/palm/applications/a'}),row={source,image:'data:image/png;base64,YWJj'};
  assert.equal(validIcon(row,source),true);assert.equal(validIcon(row,'different asset'),false);
  assert.equal(validIcon({...row,image:'http://outside/image.png'},source),false);
  assert.equal(validIcon({...row,image:'data:image/png;base64,'+'A'.repeat(400001)},source),false);
});
