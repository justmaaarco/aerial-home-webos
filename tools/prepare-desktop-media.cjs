'use strict';
// Fetch only catalogue-listed Apple SDR footage; encode local browser-compatible AV1 WebM.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),crypto=require('node:crypto'),{Readable}=require('node:stream'),{pipeline}=require('node:stream/promises'),{execFile}=require('node:child_process'),{promisify}=require('node:util');
const run=promisify(execFile),root=path.resolve(__dirname,'..'),context={window:{}};
vm.runInNewContext(fs.readFileSync(path.join(root,'app/catalogue.js'),'utf8'),context);
const catalogue=context.window.APPLE_AERIALS.scenes;
const argv=process.argv.slice(2),arg=name=>argv[argv.indexOf(name)+1],has=name=>argv.includes(name);
const destination=path.resolve(has('--dest')?arg('--dest'):path.join(root,'dist/aerial-desktop'));
const cache=path.resolve(has('--cache')?arg('--cache'):path.join(root,'dist/desktop-source-cache'));
if(!fs.existsSync(path.join(destination,'project.json')))throw Error('Build a dedicated desktop package first.');
const ids=has('--ids')?arg('--ids').split(','):null;
const selected=ids?ids.map(id=>catalogue.find(s=>s.id===id)):['Iceland Coast','Iceland Snow Caps','Bumpheads'].map(title=>catalogue.find(s=>s.title===title));
if(selected.some(s=>!s))throw Error('Unknown catalogue selection.');
const safeSource=scene=>scene.variants.filter(v=>v.range==='SDR'&&v.height===2160&&v.fps<=60&&v.available!==false).sort((a,b)=>(a.fps||30)-(b.fps||30))[0];
const entries=selected.map(scene=>{const variant=safeSource(scene);if(!variant||!/^[a-zA-Z0-9_-]+$/.test(scene.id))throw Error('No normal-speed 4K SDR source: '+scene.title);const url=new URL(variant.url);if(!['http:','https:'].includes(url.protocol)||!['sylvan.apple.com','sylvan.apple.com.edgesuite.net','a1.phobos.apple.com'].includes(url.hostname))throw Error('Source host not allowed');return {scene,variant,url:url.href};});
fs.mkdirSync(cache,{recursive:true});fs.mkdirSync(path.join(destination,'videos'),{recursive:true});
async function probe(file){const r=await run('ffprobe',['-v','error','-show_streams','-show_format','-of','json',file],{maxBuffer:1024*1024});return JSON.parse(r.stdout);}
async function sha256(file){const hash=crypto.createHash('sha256');for await(const chunk of fs.createReadStream(file))hash.update(chunk);return hash.digest('hex');}
(async()=>{
  const playlist=[],manifest=[];
  for(const {scene,variant,url} of entries){
    const source=path.join(cache,scene.id+'.mov'),output=path.join(destination,'videos',scene.id+'.webm');
    if(!fs.existsSync(source)){
      console.log('Downloading '+scene.title+' (4K SDR)');
      const response=await fetch(url,{signal:AbortSignal.timeout(120000)});if(!response.ok)throw Error('Apple download returned '+response.status);
      if(Number(response.headers.get('content-length'))>2*1024**3)throw Error('Source exceeds 2 GiB per-clip limit');
      await pipeline(Readable.fromWeb(response.body),fs.createWriteStream(source+'.part'));fs.renameSync(source+'.part',source);
    }
    const input=await probe(source),stream=input.streams.find(s=>s.codec_type==='video');
    if(stream?.width!==3840||stream?.height!==2160||stream.color_transfer==='smpte2084'||stream.color_transfer==='arib-std-b67')throw Error('Unexpected source dimensions or HDR transfer');
    if(!fs.existsSync(output)){
      console.log('Preparing '+scene.title+' (AV1 WebM, 4K, high-quality local encode)');
      await run('ffmpeg',['-hide_banner','-loglevel','warning','-y','-i',source,'-map','0:v:0','-an','-c:v','av1_nvenc','-preset','p5','-rc','vbr','-cq','17','-b:v','0','-pix_fmt','p010le','-f','webm',output+'.part'],{maxBuffer:1024*1024,timeout:600000});fs.renameSync(output+'.part',output);
    }
    const encoded=await probe(output),video=encoded.streams.find(s=>s.codec_type==='video');
    if(video?.codec_name!=='av1'||video.width!==3840||video.height!==2160||Math.abs(Number(encoded.format.duration)-Number(input.format.duration))>2)throw Error('Prepared video verification failed');
    playlist.push({id:scene.id,title:scene.title,category:scene.category,file:'videos/'+scene.id+'.webm'});
    manifest.push({id:scene.id,title:scene.title,source:url,sourceFormat:variant.key,sourceVideo:stream,preparedVideo:video,durationSeconds:Number(encoded.format.duration),bytes:fs.statSync(output).size,sha256:await sha256(output)});
    fs.writeFileSync(path.join(destination,'playlist.js'),'window.AERIAL_DESKTOP_PLAYLIST='+JSON.stringify(playlist)+';\n');
    fs.writeFileSync(path.join(destination,'media-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
    console.log('Verified '+scene.title+': '+video.width+'x'+video.height+', '+video.pix_fmt+', '+Math.round(Number(encoded.format.duration))+' sec');
  }
  fs.writeFileSync(path.join(destination,'playlist.js'),'window.AERIAL_DESKTOP_PLAYLIST='+JSON.stringify(playlist)+';\n');
  fs.writeFileSync(path.join(destination,'media-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
  console.log('Prepared '+playlist.length+' full-length local aerials.');
})().catch(e=>{console.error(e.message);process.exitCode=1;});
