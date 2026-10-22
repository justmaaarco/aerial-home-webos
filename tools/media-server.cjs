'use strict';
// Optional LAN helper. Retime Apple's 240 fps Mac files losslessly for TV playback.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto'),vm=require('node:vm');
const {spawn}=require('node:child_process');
const scope={window:{}};vm.runInNewContext(fs.readFileSync(path.resolve(__dirname,'../app/catalogue.js'),'utf8'),scope);
const scenes=new Map(scope.window.APPLE_AERIALS.scenes.map(s=>[s.id,s]));
const cache=path.resolve(process.env.AERIAL_CACHE || path.join(os.homedir(),'.cache/aerial-home'));
const port=Number(process.env.AERIAL_PORT || 8794),limit=Number(process.env.AERIAL_CACHE_MB || 3072)*1024*1024;
fs.mkdirSync(cache,{recursive:true,mode:0o700});
// A previous interrupted preparation must not leave unbounded orphan files.
for(const name of fs.readdirSync(cache)){if(/^[a-f0-9]{64}\.part\.mp4$/.test(name)&&fs.lstatSync(path.join(cache,name)).isFile())fs.unlinkSync(path.join(cache,name));}
const jobs=new Map(),queue=[],readers=new Map();let working=false;
const keyOf=url=>crypto.createHash('sha256').update(url).digest('hex');
function safeSource(scene){return scene.variants.find(v=>v.height===2160&&v.fps>60&&v.range==='SDR'&&new URL(v.url).hostname==='sylvan.apple.com');}
function cacheEntries(){return fs.readdirSync(cache).filter(n=>/^[a-f0-9]{64}\.mp4$/.test(n)).map(name=>({name,stat:fs.lstatSync(path.join(cache,name))})).filter(f=>f.stat.isFile()).map(({name,stat})=>({name,size:stat.size,mtimeMs:stat.mtimeMs}));}
function prune(reserve=0){
  const files=cacheEntries().sort((a,b)=>a.mtimeMs-b.mtimeMs);
  let total=files.reduce((s,f)=>s+f.size,0);
  for(const f of files){if(total+reserve<=limit)break;if(readers.get(f.name))continue;fs.unlinkSync(path.join(cache,f.name));total-=f.size;}
  return total+reserve<=limit;
}
function execute(command,args,timeout,onProgress){
  return new Promise((resolve,reject)=>{
    const p=spawn(command,args,{stdio:['ignore','pipe','pipe']});let out='',err='';
    const timer=setTimeout(()=>p.kill('SIGKILL'),timeout);
    p.stdout.on('data',chunk=>{out=(out+chunk).slice(-16000);onProgress?.(String(chunk));});
    p.stderr.on('data',chunk=>{err=(err+chunk).slice(-2000);});
    p.on('error',error=>{clearTimeout(timer);reject(error);});
    p.on('close',code=>{clearTimeout(timer);code===0?resolve(out):reject(new Error(err||'Media preparation failed'));});
  });
}
async function work(){
  if(working||!queue.length)return;working=true;
  const job=queue.shift(),target=path.join(cache,job.key+'.mp4'),temp=path.join(cache,job.key+'.part.mp4');
  job.state='preparing';
  try {
    prune(Math.min(limit,768*1024*1024));
    const factor=String(Math.round(job.variant.fps/(30000/1001)));
    // Stream copy preserves every coded picture, bit depth and colour information.
    // Only container timestamps and HEVC timing metadata are adjusted (no decoder/encoder).
    await execute('ffmpeg',['-nostdin','-y','-v','error','-rw_timeout','15000000','-itsscale',factor,'-i',job.variant.url,'-map','0:v:0','-c:v','copy','-an','-bsf:v','hevc_metadata=tick_rate=30000/1001:level=5.1','-tag:v','hvc1','-movflags','+faststart','-fs',String(limit+1),'-progress','pipe:1',temp],480000,chunk=>{
      const match=chunk.match(/total_size=(\d+)/);if(match)job.bytes=Number(match[1]);
    });
    const probe=JSON.parse(await execute('ffprobe',['-v','error','-select_streams','v:0','-show_entries','stream=width,height,codec_name,avg_frame_rate,pix_fmt,color_transfer','-of','json',temp],15000));
    const stream=probe.streams?.[0],rate=stream?.avg_frame_rate?.split('/').map(Number);
    if(!stream||stream.width!==3840||stream.height!==2160||stream.codec_name!=='hevc'||rate[0]/rate[1]>60||!stream.pix_fmt.includes('10'))throw new Error('Prepared video did not pass 4K/10-bit/frame-rate verification');
    const size=fs.statSync(temp).size;if(size>limit)throw new Error('Video exceeds configured cache limit');
    if(!prune(size))throw new Error('Cache is busy with a playing video; retry after it finishes');
    fs.renameSync(temp,target);job.state='ready';job.bytes=size;job.verified=stream;
  }catch(error){job.state='error';job.error=error.message.slice(-250);job.failedAt=Date.now();try{fs.unlinkSync(temp);}catch(_) {}}
  finally{working=false;work();}
}
function json(res,status,data){res.writeHead(status,{'Content-Type':'application/json','Cache-Control':'no-store'});res.end(JSON.stringify(data));}
const server=http.createServer((req,res)=>{
  // Public media only; no credentials, TV controls, directory listings or arbitrary URLs.
  res.setHeader('Access-Control-Allow-Origin','*');res.setHeader('Access-Control-Allow-Methods','GET,HEAD,OPTIONS');
  if(req.method==='OPTIONS'){res.writeHead(204).end();return;}
  if(!['GET','HEAD'].includes(req.method)){json(res,405,{error:'Read-only service'});return;}
  let url;try{url=new URL(req.url,'http://localhost');}catch(_){res.writeHead(400).end();return;}
  if(url.pathname==='/api/status'){const files=cacheEntries();json(res,200,{version:'0.2.0',pending:queue.length,working,cached:files.length,cachedBytes:files.reduce((n,f)=>n+f.size,0)});return;}
  const prepare=url.pathname.match(/^\/api\/prepare\/([a-zA-Z0-9-]+)$/);
  if(prepare){
    const scene=scenes.get(prepare[1]),variant=scene&&safeSource(scene);if(!variant){json(res,404,{error:'No compatible Apple source'});return;}
    const key=keyOf(variant.url),target=path.join(cache,key+'.mp4');
    if(fs.existsSync(target)){const now=new Date();fs.utimesSync(target,now,now);json(res,200,{state:'ready',url:'/media/'+key+'.mp4',height:2160,range:'SDR',fps:30000/1001,lossless:true});return;}
    let job=jobs.get(key);
    if(job?.state==='error'&&Date.now()-job.failedAt>30000){jobs.delete(key);job=null;}
    if(job?.state==='ready'){jobs.delete(key);job=null;}
    if(!job){if(queue.length>=3){json(res,429,{state:'busy'});return;}job={key,variant,state:'queued',bytes:0};jobs.set(key,job);queue.push(job);work();}
    json(res,200,{state:job.state,bytes:job.bytes,error:job.error});return;
  }
  const media=url.pathname.match(/^\/media\/([a-f0-9]{64})\.mp4$/);
  if(media){
    const name=media[1]+'.mp4',file=path.join(cache,name);let stat;try{stat=fs.lstatSync(file);if(!stat.isFile())throw Error('Not a regular cache file');}catch(_){res.writeHead(404).end();return;}
    const size=stat.size;let start=0,end=size-1,status=200;
    if(req.headers.range){
      const match=/^bytes=(\d*)-(\d*)$/.exec(req.headers.range);
      if(!match){res.writeHead(416,{'Content-Range':'bytes */'+size}).end();return;}
      if(match[1]===''){start=Math.max(0,size-Number(match[2]));}else{start=Number(match[1]);if(match[2])end=Math.min(end,Number(match[2]));}
      if(!Number.isSafeInteger(start)||start<0||start>end||start>=size){res.writeHead(416,{'Content-Range':'bytes */'+size}).end();return;}
      status=206;res.setHeader('Content-Range','bytes '+start+'-'+end+'/'+size);
    }
    res.writeHead(status,{'Content-Type':'video/mp4','Accept-Ranges':'bytes','Content-Length':end-start+1,'Cache-Control':'private, max-age=3600'});
    if(req.method==='HEAD'){res.end();return;}
    readers.set(name,(readers.get(name)||0)+1);let released=false;
    const release=()=>{if(released)return;released=true;readers.set(name,Math.max(0,(readers.get(name)||1)-1));};
    const stream=fs.createReadStream(file,{start,end});stream.on('error',()=>{release();res.destroy();});stream.on('end',release);res.on('close',()=>{release();stream.destroy();});stream.pipe(res);return;
  }
  res.writeHead(404).end();
});
server.listen(port,process.env.AERIAL_BIND || '127.0.0.1',()=>console.log('Aerial media helper on port '+server.address().port+'; cache limit '+Math.round(limit/1024/1024)+' MiB'));
