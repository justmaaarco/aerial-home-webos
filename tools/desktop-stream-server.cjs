'use strict';
// Loopback-only public-media service: catalogue IDs only, no commands or arbitrary URLs from clients.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),{spawn}=require('node:child_process');
const {sources,argumentsFor}=require(fs.existsSync(path.join(__dirname,'catalogue-core.cjs'))?'./catalogue-core.cjs':'../desktop/catalogue-core.cjs');
const scope={window:{}},catalogueFile=fs.existsSync(path.join(__dirname,'catalogue.js'))?path.join(__dirname,'catalogue.js'):path.join(__dirname,'../app/catalogue.js');
vm.runInNewContext(fs.readFileSync(catalogueFile,'utf8'),scope);
const scenes=new Map(scope.window.APPLE_AERIALS.scenes.filter(s=>/^[a-zA-Z0-9_-]+$/.test(s.id)&&sources(s).length).map(s=>[s.id,s]));
const port=Number(process.env.AERIAL_DESKTOP_PORT||5194),cache=path.resolve(process.env.AERIAL_DESKTOP_CACHE||path.join(__dirname,'../cache'));
const seed=path.resolve(process.env.AERIAL_DESKTOP_SEED||path.join(__dirname,'../wallpaper/videos'));
const limit=4*1024**3,maxEntry=limit/2,readers=new Map(),failures=new Map();let worker=null,lastResult=null;
const clients=new Map();
fs.mkdirSync(cache,{recursive:true});
// Legacy cache can contain a 1080p fallback for a 4K scene. Only new quality-
// restricted encodes are playable; both generations still count toward pruning.
const cacheName=id=>id+'.qhd.webm';
const cacheFiles=()=>fs.readdirSync(cache).filter(n=>/^[a-zA-Z0-9_-]+(?:\.qhd)?\.webm$/.test(n)).flatMap(name=>{try{const stat=fs.lstatSync(path.join(cache,name));return stat.isFile()?[{name,stat}]:[];}catch(_){return [];}});
function readyIds(){
  const ids=new Set();
  for(const dir of [seed,cache]){
    let names=[];try{names=fs.readdirSync(dir);}catch(_){}
    for(const name of names){
      const suffix=dir===cache?'.qhd.webm':'.webm',id=name.endsWith(suffix)?name.slice(0,-suffix.length):'';if(!scenes.has(id))continue;
      try{const stat=fs.lstatSync(path.join(dir,name));if(stat.isFile()&&stat.size>0)ids.add(id);}catch(_){}
    }
  }
  return [...ids];
}
function prune(reserve=0){const files=cacheFiles().sort((a,b)=>a.stat.mtimeMs-b.stat.mtimeMs);let total=files.reduce((n,f)=>n+f.stat.size,0);for(const f of files){if(total+reserve<=limit)break;if(readers.get(f.name))continue;try{fs.unlinkSync(path.join(cache,f.name));total-=f.stat.size;}catch(_){}}return total+reserve<=limit;}
// Only this service's known temporary files are eligible for cleanup after an interrupted run.
for(const name of fs.readdirSync(cache))if(/^[a-zA-Z0-9_-]+(?:\.qhd)?\.webm\.(part|index)$/.test(name)&&fs.lstatSync(path.join(cache,name)).isFile())fs.unlinkSync(path.join(cache,name));
function json(res,status,data){res.writeHead(status,{'Content-Type':'application/json','Cache-Control':'no-store'});res.end(JSON.stringify(data));}
function cached(res,req,file){
  const stat=fs.statSync(file),size=stat.size,name=path.basename(file);let start=0,end=size-1,status=200;
  if(req.headers.range){const m=/^bytes=(\d+)-(\d*)$/.exec(req.headers.range);if(!m){res.writeHead(416,{'Content-Range':'bytes */'+size}).end();return;}start=Number(m[1]);end=m[2]?Math.min(Number(m[2]),end):end;if(!Number.isSafeInteger(start)||start>end||start>=size){res.writeHead(416,{'Content-Range':'bytes */'+size}).end();return;}status=206;res.setHeader('Content-Range','bytes '+start+'-'+end+'/'+size);}
  res.writeHead(status,{'Content-Type':'video/webm','Content-Length':end-start+1,'Accept-Ranges':'bytes'});if(req.method==='HEAD'){res.end();return;}
  readers.set(name,(readers.get(name)||0)+1);const s=fs.createReadStream(file,{start,end});let released=false;
  const release=()=>{if(released)return;released=true;readers.set(name,Math.max(0,(readers.get(name)||1)-1));s.destroy();};res.on('close',release);s.on('error',()=>res.destroy());s.pipe(res);
}
const server=http.createServer((req,res)=>{
  const origin=req.headers.origin,own='http://127.0.0.1:'+port;
  if(!['127.0.0.1:'+port,'localhost:'+port].includes(req.headers.host)||origin&&!['null',own,'http://localhost:'+port].includes(origin)){json(res,403,{error:'Local wallpaper requests only'});return;}
  res.setHeader('Access-Control-Allow-Origin',origin||'*');res.setHeader('Access-Control-Allow-Methods','GET,HEAD,POST,OPTIONS');res.setHeader('Access-Control-Allow-Headers','Range,Content-Type');
  if(req.method==='OPTIONS'){res.writeHead(204).end();return;}
  if(req.method==='POST'&&req.url==='/api/client'){
    let body='';req.on('data',c=>{body+=c;if(body.length>2048)req.destroy();});req.on('end',()=>{try{const d=JSON.parse(body);if(typeof d.client!=='string'||!/^[a-z0-9-]{1,40}$/.test(d.client)||!['selected','properties'].includes(d.event)||!/^(?:[a-f0-9]{12}|source)$/.test(d.build)||d.scene!==null&&!scenes.has(d.scene)||![d.nextRequests,d.propertyEvents].every(n=>Number.isSafeInteger(n)&&n>=0&&n<=1000000000))throw Error('Invalid status');
      if(!clients.has(d.client)&&clients.size>=8)clients.delete(clients.keys().next().value);
      clients.set(d.client,{client:d.client,event:d.event,build:String(d.build).slice(0,20),scene:d.scene,paused:d.paused===true,loading:d.loading===true,nextRequests:d.nextRequests,propertyEvents:d.propertyEvents,nextControl:d.nextControl===true,updatedAt:Date.now()});json(res,200,{ok:true});
    }catch(_){json(res,400,{error:'Invalid client status'});}});return;
  }
  if(!['GET','HEAD'].includes(req.method)){json(res,405,{error:'Read-only media service'});return;}
  let u;try{u=new URL(req.url,own);}catch(_){json(res,400,{error:'Invalid request'});return;}
  if(u.pathname==='/api/status'){const files=cacheFiles();json(res,200,{service:'aerial-desktop-catalogue',schema:1,catalogue:scenes.size,cacheBytes:files.reduce((n,f)=>n+f.stat.size,0),cacheLimitBytes:limit,worker:worker?{id:worker.id,title:worker.title}:null,lastResult,clients:[...clients.values()]});return;}
  const match=/^\/video\/([a-zA-Z0-9_-]+)\.webm$/.exec(u.pathname),scene=match&&scenes.get(match[1]);
  const info=/^\/api\/info\/([a-zA-Z0-9_-]+)$/.exec(u.pathname);
  if(info&&scenes.has(info[1])){const ready=readyIds();json(res,200,{cached:ready.includes(info[1]),busy:!!worker,ready});return;}
  if(!scene){json(res,404,{error:'Unknown catalogue ID'});return;}
  for(const dir of [seed,cache]){const file=path.join(dir,dir===cache?cacheName(scene.id):scene.id+'.webm');try{const stat=fs.lstatSync(file);if(stat.isFile()&&stat.size>0){cached(res,req,file);return;}}catch(_){}}
  if(req.method==='HEAD'){res.writeHead(200,{'Content-Type':'video/webm'}).end();return;}
  const offset=Number(u.searchParams.get('t')||0);if(!Number.isFinite(offset)||offset<0||offset>7200){json(res,400,{error:'Invalid playback position'});return;}
  if(worker){json(res,503,{error:'Another aerial is preparing; retry shortly'});return;}
  const candidates=sources(scene).filter(v=>!failures.get(v.url)||Date.now()-failures.get(v.url)>24*3600000);
  if(!candidates.length){json(res,404,{error:'No reachable SDR source remains for this aerial'});return;}
  const job={id:scene.id,title:scene.title,process:null,closed:false,complete:false};worker=job;
  let output=null,eligible=false,bytes=0,temp=path.join(cache,cacheName(scene.id)+'.part');
  const cleanup=()=>{if(output){job.process?.stdout?.unpipe(output);output.destroy();output=null;}try{fs.unlinkSync(temp);}catch(_){}};
  res.on('close',()=>{job.closed=true;if(!job.complete){job.process?.kill();cleanup();if(worker===job)worker=null;}});
  function attempt(index){
    if(job.closed)return;if(index>=candidates.length){lastResult={id:scene.id,state:'unavailable'};if(worker===job)worker=null;json(res,502,{error:'Apple sources for this aerial could not be opened'});return;}
    const variant=candidates[index],p=spawn(process.env.AERIAL_DESKTOP_FFMPEG||'ffmpeg',argumentsFor(variant,offset),{stdio:['ignore','pipe','pipe'],windowsHide:true});job.process=p;let started=false,error='';
    p.stderr.on('data',c=>{error=(error+c).slice(-1500);});
    p.on('error',e=>{error=e.message;});
    const watchdog=setTimeout(()=>{if(!started)p.kill();},30000);
    p.stdout.once('data',chunk=>{
      if(job.closed)return;started=true;clearTimeout(watchdog);res.writeHead(200,{'Content-Type':'video/webm','Cache-Control':'no-store','Accept-Ranges':'none'});res.write(chunk);
      eligible=offset===0;bytes=chunk.length;
      if(eligible){cleanup();output=fs.createWriteStream(temp);output.on('error',()=>{eligible=false;cleanup();});output.write(chunk);p.stdout.pipe(output);}
      p.stdout.on('data',c=>{bytes+=c.length;if(eligible&&bytes>maxEntry){eligible=false;cleanup();}});
      p.stdout.pipe(res,{end:false});
    });
    p.on('close',code=>{
      clearTimeout(watchdog);if(job.closed)return;
      if(code!==0||!started){failures.set(variant.url,Date.now());cleanup();if(!started){attempt(index+1);return;}lastResult={id:scene.id,state:'error',detail:error.slice(-250)};if(worker===job)worker=null;res.destroy();return;}
      job.complete=true;lastResult={id:scene.id,state:'played',height:variant.height,retimed:variant.fps>60};
      const save=()=>{
        res.end();
        if(!eligible||!fs.existsSync(temp)||!prune(bytes*2)){cleanup();if(worker===job)worker=null;return;}
        // The streamed WebM has no seek index. Losslessly remux it before publishing a cache entry.
        const indexed=path.join(cache,cacheName(scene.id)+'.index');
        const p=spawn(process.env.AERIAL_DESKTOP_FFMPEG||'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-y','-i',temp,'-map','0:v:0','-c','copy','-f','webm',indexed],{stdio:'ignore',windowsHide:true});job.process=p;p.on('error',()=>{});
        p.on('close',code=>{try{if(code===0)fs.renameSync(indexed,path.join(cache,cacheName(scene.id)));else fs.unlinkSync(indexed);}catch(_){}cleanup();if(worker===job)worker=null;});
      };
      if(output&&!output.writableFinished)output.once('finish',save);else save();
    });
  }
  attempt(0);
});
server.listen(port,'127.0.0.1',()=>console.log('Aerial catalogue service: '+scenes.size+' entries on loopback port '+port));
function shutdown(){worker?.process?.kill();server.close(()=>process.exit(0));setTimeout(()=>process.exit(0),1000).unref();}
process.on('SIGTERM',shutdown);process.on('SIGINT',shutdown);
