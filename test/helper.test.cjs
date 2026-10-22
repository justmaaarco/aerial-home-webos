'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn}=require('node:child_process');
test('media helper serves bounded ranges and rejects arbitrary paths, sources and writes',async()=>{
  const cache=fs.mkdtempSync(path.join(os.tmpdir(),'aerial-helper-test-')),name='a'.repeat(64)+'.mp4',file=path.join(cache,name);fs.writeFileSync(file,'0123456789');
  const orphan=path.join(cache,'b'.repeat(64)+'.part.mp4');fs.writeFileSync(orphan,'unfinished');
  const child=spawn(process.execPath,[path.resolve(__dirname,'../tools/media-server.cjs')],{env:{...process.env,AERIAL_PORT:'0',AERIAL_BIND:'127.0.0.1',AERIAL_CACHE:cache},stdio:['ignore','pipe','pipe']});
  try {
    const port=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('Helper startup timeout')),5000);child.stdout.on('data',chunk=>{const match=String(chunk).match(/port (\d+)/);if(match){clearTimeout(timer);resolve(Number(match[1]));}});child.once('error',reject);});
    const base='http://127.0.0.1:'+port;
    assert.equal(fs.existsSync(orphan),false);
    const status=await (await fetch(base+'/api/status')).json();assert.equal(status.cached,1);assert.equal(status.cachedBytes,10);
    let r=await fetch(base+'/media/'+name,{headers:{Range:'bytes=2-5'}});assert.equal(r.status,206);assert.equal(r.headers.get('content-range'),'bytes 2-5/10');assert.equal(await r.text(),'2345');
    r=await fetch(base+'/media/'+name,{method:'HEAD'});assert.equal(r.status,200);assert.equal(r.headers.get('content-length'),'10');assert.equal(await r.text(),'');
    assert.equal((await fetch(base+'/media/'+name,{headers:{Range:'bytes=100-200'}})).status,416);
    assert.equal((await fetch(base+'/api/prepare/unknown')).status,404);
    assert.equal((await fetch(base+'/api/prepare/http%3A%2F%2Fevil.test')).status,404);
    assert.equal((await fetch(base+'/%2e%2e/%2e%2e/etc/passwd')).status,404);
    assert.equal((await fetch(base+'/api/status',{method:'POST'})).status,405);
  }finally{child.kill();await new Promise(resolve=>child.once('close',resolve));fs.unlinkSync(file);fs.rmdirSync(cache);}
});
