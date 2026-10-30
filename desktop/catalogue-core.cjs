'use strict';
function sources(scene){
  const seen=new Set();
  return (scene?.variants||[]).filter(v=>{
    let u;try{u=new URL(v.url);}catch(_){return false;}
    if(v.range!=='SDR'||v.available===false||!['http:','https:'].includes(u.protocol)||!['sylvan.apple.com','a1.phobos.apple.com'].includes(u.hostname)||u.username||u.password||seen.has(u.href))return false;
    seen.add(u.href);return true;
  }).sort((a,b)=>b.height-a.height||(a.fps>60)-(b.fps>60));
}
function argumentsFor(variant,offset=0){
  const factor=variant.fps>60?Math.round(variant.fps/(30000/1001)):1,fps=variant.fps>60?'30000/1001':variant.fps||30;
  const args=['-nostdin','-hide_banner','-loglevel','error','-xerror','-rw_timeout','15000000','-hwaccel','cuda','-hwaccel_output_format','cuda'];
  if(offset>0)args.push('-ss',String(offset/factor));
  if(factor!==1)args.push('-itsscale',String(factor));
  args.push('-i',variant.url,'-map','0:v:0','-an','-vf','realtime=speed=1.15','-c:v','av1_nvenc','-preset','p4','-rc','vbr','-cq','22','-b:v','0','-r',String(fps),'-f','webm','-cluster_time_limit','1000','-live','1','pipe:1');
  return args;
}
module.exports={sources,argumentsFor};
