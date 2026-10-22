'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const source=fs.readFileSync(require.resolve('../app/bridge.js'),'utf8');
function setup(reply) {
  const calls=[],instances=[];
  class PalmServiceBridge {
    constructor(){instances.push(this);this.cancelled=false;}
    cancel(){this.cancelled=true;}
    call(uri,payload){calls.push({uri,payload:JSON.parse(payload)});queueMicrotask(()=>this.onservicecallback(JSON.stringify(reply(uri,JSON.parse(payload)))));}
  }
  const window={PalmServiceBridge};vm.runInNewContext(source,{window,Set,Map,Promise,Error,Object,JSON,setTimeout,clearTimeout});return {TV:window.TV,calls,instances};
}
test('Luna errors propagate and bridge subscriptions are released',async()=>{
  const {TV,instances}=setup(()=>({returnValue:false,errorText:'Permission denied'}));
  await assert.rejects(TV.call('luna://test'),/Permission denied/);assert.equal(instances[0].cancelled,true);
});
test('app inventory falls back to HBC and includes HDMI discovered from EIM',async()=>{
  const {TV,calls}=setup((uri,p)=>{
    if(uri.includes('listLaunchPoints'))return {returnValue:false,errorText:'Denied'};
    return {returnValue:true,stdoutString:JSON.stringify(p.command.includes('listLaunchPoints')?{launchPoints:[{id:'netflix',title:'Netflix',icon:'/media/cryptofs/apps/usr/palm/applications/netflix/icon.png'}]}:{devices:[{appId:'com.webos.app.hdmi3',label:'HDMI 3'}]})};
  });
  const apps=await TV.listApps();assert.equal(apps.length,3);assert.equal(apps[1].input,true);assert.equal(apps[2].id,'settings');assert.equal(calls.length,3);
});
test('artwork reader rejects shell characters, traversal and unrelated files',async()=>{
  const {TV,calls}=setup(()=>({returnValue:true,stdoutString:'YWJj'}));
  for(const icon of ['/etc/shadow','/media/cryptofs/apps/usr/palm/applications/a/../../secret.png',"/usr/palm/applications/a/$(id).png",'/usr/palm/applications/a/icon.png;id'])assert.equal(await TV.icon({icon}),null);
  assert.equal(calls.length,0);
  assert.equal(await TV.icon({icon:'/media/cryptofs/apps/usr/palm/applications/a/icon.png'}),'data:image/png;base64,YWJj');assert.equal(calls.length,1);
});
test('first app inventory can finish without waiting for HDMI discovery',async()=>{
  const {TV,calls}=setup((uri,p)=>{
    if(uri.includes('listLaunchPoints'))return {returnValue:true,launchPoints:[{id:'netflix',title:'Netflix'}]};
    throw Error('HDMI must not be requested during first inventory');
  });
  const apps=await TV.listApps({includeInputs:false});assert.equal(apps[0].id,'netflix');assert.equal(calls.length,1);
});
