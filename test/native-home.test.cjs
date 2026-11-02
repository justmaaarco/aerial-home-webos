'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict');
const {PowerState}=require('../tools/native-home/watch.cjs');
const {APP,openNative}=require('../tools/native-home/open-native.cjs');

test('native power routing launches once per real wake, not screen saver or screen off',()=>{
  const p=new PowerState(true);
  const sequence=['Active','Active','Screen Saver','Active','Screen Off','Active','Active Standby','Active','Active'];
  assert.deepEqual(sequence.map(state=>p.update({state})),[true,false,false,false,false,false,false,true,false]);
  const quiet=new PowerState(false);
  assert.equal(quiet.update({state:'Active'}),false);
  assert.equal(quiet.update({state:'Active',processing:'Request Power Off'}),false);
  assert.equal(quiet.update({state:'Active',returnValue:false}),false);
  assert.equal(quiet.update({state:'Active'}),true);
});

test('native Home resets its view before foregrounding only the native app',async()=>{
  const calls=[];
  const result=await openNative({validate(){calls.push('validate');},async signal(){calls.push('reset');return{returnValue:true};},async request(uri,payload){calls.push('launch');assert.equal(uri,'luna://com.webos.applicationManager/launch');assert.deepEqual(payload,{id:APP});assert.equal(APP,'io.github.justmaaarco.aerialhome.nativeprobe');return{returnValue:true};}});
  assert.deepEqual(calls,['validate','reset','launch']);assert.equal(result.returnValue,true);
});

test('a cancelled power transition never launches after the reset completes',async()=>{
  let active=true,launches=0;
  const result=await openNative({validate(){},canLaunch:()=>active,async signal(){active=false;return{returnValue:true};},async request(){launches++;return{returnValue:true};}});
  assert.equal(result.returnValue,false);assert.equal(launches,0);
});

test('native Home reset failure does not fall through to another launcher or input',async()=>{
  let launches=0;
  const result=await openNative({validate(){},async signal(){return{returnValue:false,errorText:'reset failed'};},async request(){launches++;}});
  assert.equal(result.errorText,'reset failed');assert.equal(launches,0);
});
