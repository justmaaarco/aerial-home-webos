'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),vm=require('node:vm'),{spawn}=require('node:child_process');
const {sources,argumentsFor}=require('../desktop/catalogue-core.cjs');
const context={window:{}};vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../app/catalogue.js'),'utf8'),context);const catalogue=context.window.APPLE_AERIALS.scenes,scenes=catalogue.filter(s=>sources(s).length);
test('desktop retains only 1440p or higher sources, excludes 52 HD-only scenes and corrects Mac timing',()=>{
  assert.equal(catalogue.length,216);assert.equal(scenes.length,164);
  let hd=0,retimed=0;for(const scene of scenes){const variants=sources(scene);assert.ok(variants.length,scene.id);const first=variants[0];if(first.height<2160)hd++;if(first.fps>60){retimed++;const args=argumentsFor(first,80);assert.equal(args[args.indexOf('-itsscale')+1],'8');assert.equal(args[args.indexOf('-ss')+1],'10');}assert.ok(variants.every(v=>v.range==='SDR'));}
  assert.equal(hd,0);assert.equal(retimed,38);
  assert.ok(scenes.every(s=>sources(s).every(v=>v.height>=1440)));
  assert.equal(sources(catalogue.find(s=>s.id==='b3-1')).length,0);
  const url='http://sylvan.apple.com/test.mov';
  assert.equal(sources({variants:[{url,height:1440,range:'SDR'}]}).length,1);
  assert.equal(sources({variants:[{url,height:1080,range:'SDR',key:'url-2K-SDR'}]}).length,0);
  assert.equal(sources({variants:[{url:'http://sylvan.apple.com.evil.invalid/a',range:'SDR'},{url:'file:///tmp/a',range:'SDR'},{url:'https://user@sylvan.apple.com/a',range:'SDR'}]}).length,0);
});
test('catalogue helper is loopback-only, read-only, range bounded and never accepts client URLs',async t=>{
  const temp=fs.mkdtempSync(path.join(os.tmpdir(),'aerial-desktop-test-')),seed=path.join(temp,'seed'),cache=path.join(temp,'cache'),port=19000+Math.floor(Math.random()*9000);fs.mkdirSync(seed);fs.writeFileSync(path.join(seed,scenes[0].id+'.webm'),Buffer.from([0x1a,0x45,0xdf,0xa3,1,2,3,4]));
  const child=spawn(process.execPath,[path.join(__dirname,'../tools/desktop-stream-server.cjs')],{env:{...process.env,AERIAL_DESKTOP_PORT:String(port),AERIAL_DESKTOP_SEED:seed,AERIAL_DESKTOP_CACHE:cache},stdio:['ignore','pipe','pipe'],windowsHide:true});
  t.after(async()=>{child.kill();await new Promise(resolve=>{if(child.exitCode!==null)resolve();else child.once('close',resolve);});fs.rmSync(temp,{recursive:true,force:true});});
  await new Promise((resolve,reject)=>{child.stdout.once('data',resolve);child.once('error',reject);child.once('exit',code=>{if(code)reject(Error('Helper exited '+code));});});
  const base='http://127.0.0.1:'+port;
  assert.equal((await (await fetch(base+'/api/status')).json()).catalogue,164);
  const info=await (await fetch(base+'/api/info/'+scenes[0].id)).json();assert.equal(info.cached,true);assert.deepEqual(info.ready,[scenes[0].id]);
  const range=await fetch(base+'/video/'+scenes[0].id+'.webm',{headers:{Range:'bytes=1-3'}});assert.equal(range.status,206);assert.equal((await range.arrayBuffer()).byteLength,3);
  assert.equal((await fetch(base+'/video/'+scenes[0].id+'.webm',{headers:{Range:'bytes=99-100'}})).status,416);
  assert.equal((await fetch(base+'/video/not-a-catalogue-id.webm?url=https://example.com/')).status,404);
  assert.equal((await fetch(base+'/video/b3-1.webm')).status,404);
  assert.equal((await fetch(base+'/api/info/b3-1')).status,404);
  // Old files may have been generated after a 4K URL fell back to 1080p. Their
  // filenames alone cannot prove resolution, so do not advertise or serve them.
  const cachedId=scenes[1].id;fs.writeFileSync(path.join(cache,cachedId+'.webm'),Buffer.from('old HD cache'));
  assert.equal((await(await fetch(base+'/api/info/'+cachedId)).json()).cached,false);
  fs.writeFileSync(path.join(cache,cachedId+'.qhd.webm'),Buffer.from('quality-restricted cache'));
  const ready=await(await fetch(base+'/api/info/'+cachedId)).json();assert.equal(ready.cached,true);assert.ok(ready.ready.includes(cachedId));
  assert.equal(await(await fetch(base+'/video/'+cachedId+'.webm')).text(),'quality-restricted cache');
  assert.equal((await fetch(base+'/api/status',{method:'POST'})).status,405);
  assert.equal((await fetch(base+'/api/status',{headers:{Origin:'https://example.com'}})).status,403);
  const post=data=>fetch(base+'/api/client',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data)});
  const state={client:'test',event:'properties',build:'source',scene:scenes[0].id,paused:false,loading:false,nextRequests:1,propertyEvents:2,nextControl:true};
  assert.equal((await post({...state,scene:'not-a-catalogue-id'})).status,400);
  assert.equal((await post({...state,nextRequests:-1})).status,400);
  for(let n=0;n<10;n++)assert.equal((await post({...state,client:'test-'+n})).status,200);
  const clientStatus=await(await fetch(base+'/api/status')).json();assert.equal(clientStatus.clients.length,8);assert.equal(clientStatus.clients.at(-1).nextRequests,1);
  assert.equal((await (await fetch(base+'/api/status')).json()).worker,null);
});
test('built desktop and screensaver playlists exclude HD-only scenes',async t=>{
  const temp=fs.mkdtempSync(path.join(os.tmpdir(),'aerial-quality-build-'));
  t.after(()=>fs.rmSync(temp,{recursive:true,force:true}));
  for(const mode of ['desktop','screensaver']){
    const dir=path.join(temp,mode),args=[path.join(__dirname,'../tools/build-desktop.cjs'),dir,'--catalogue'];if(mode==='screensaver')args.push('--screensaver');
    const child=spawn(process.execPath,args,{stdio:'ignore',windowsHide:true});
    assert.equal(await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',resolve);}),0);
    const scope={window:{}};vm.runInNewContext(fs.readFileSync(path.join(dir,'playlist.js'),'utf8'),scope);
    const list=scope.window.AERIAL_DESKTOP_PLAYLIST;assert.equal(list.length,164);assert.ok(!list.some(s=>s.id==='b3-1'));
    assert.ok(list.every(s=>sources(catalogue.find(c=>c.id===s.id)).every(v=>v.height>=1440)));
  }
});
