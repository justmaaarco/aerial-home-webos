'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),vm=require('node:vm'),{spawn}=require('node:child_process');
const {sources,argumentsFor}=require('../desktop/catalogue-core.cjs');
const context={window:{}};vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../app/catalogue.js'),'utf8'),context);const scenes=context.window.APPLE_AERIALS.scenes;
test('the entire catalogue has a safe SDR source; legacy HD is retained and Mac timing is corrected',()=>{
  assert.equal(scenes.length,216);
  let hd=0,retimed=0;for(const scene of scenes){const variants=sources(scene);assert.ok(variants.length,scene.id);const first=variants[0];if(first.height<2160)hd++;if(first.fps>60){retimed++;const args=argumentsFor(first,80);assert.equal(args[args.indexOf('-itsscale')+1],'8');assert.equal(args[args.indexOf('-ss')+1],'10');}assert.ok(variants.every(v=>v.range==='SDR'));}
  assert.equal(hd,52);assert.equal(retimed,38);
  assert.equal(sources({variants:[{url:'http://sylvan.apple.com.evil.invalid/a',range:'SDR'},{url:'file:///tmp/a',range:'SDR'},{url:'https://user@sylvan.apple.com/a',range:'SDR'}]}).length,0);
});
test('catalogue helper is loopback-only, read-only, range bounded and never accepts client URLs',async t=>{
  const temp=fs.mkdtempSync(path.join(os.tmpdir(),'aerial-desktop-test-')),seed=path.join(temp,'seed'),cache=path.join(temp,'cache'),port=19000+Math.floor(Math.random()*9000);fs.mkdirSync(seed);fs.writeFileSync(path.join(seed,scenes[0].id+'.webm'),Buffer.from([0x1a,0x45,0xdf,0xa3,1,2,3,4]));
  const child=spawn(process.execPath,[path.join(__dirname,'../tools/desktop-stream-server.cjs')],{env:{...process.env,AERIAL_DESKTOP_PORT:String(port),AERIAL_DESKTOP_SEED:seed,AERIAL_DESKTOP_CACHE:cache},stdio:['ignore','pipe','pipe'],windowsHide:true});
  t.after(async()=>{child.kill();await new Promise(resolve=>{if(child.exitCode!==null)resolve();else child.once('close',resolve);});fs.rmSync(temp,{recursive:true,force:true});});
  await new Promise((resolve,reject)=>{child.stdout.once('data',resolve);child.once('error',reject);child.once('exit',code=>{if(code)reject(Error('Helper exited '+code));});});
  const base='http://127.0.0.1:'+port;
  assert.equal((await (await fetch(base+'/api/status')).json()).catalogue,216);
  assert.equal((await (await fetch(base+'/api/info/'+scenes[0].id)).json()).cached,true);
  const range=await fetch(base+'/video/'+scenes[0].id+'.webm',{headers:{Range:'bytes=1-3'}});assert.equal(range.status,206);assert.equal((await range.arrayBuffer()).byteLength,3);
  assert.equal((await fetch(base+'/video/'+scenes[0].id+'.webm',{headers:{Range:'bytes=99-100'}})).status,416);
  assert.equal((await fetch(base+'/video/not-a-catalogue-id.webm?url=https://example.com/')).status,404);
  assert.equal((await fetch(base+'/api/status',{method:'POST'})).status,405);
  assert.equal((await fetch(base+'/api/status',{headers:{Origin:'https://example.com'}})).status,403);
  assert.equal((await (await fetch(base+'/api/status')).json()).worker,null);
});
