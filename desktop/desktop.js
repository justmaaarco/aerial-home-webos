(function(){
  'use strict';
  const $=id=>document.getElementById(id),video=$('aerial'),bag=new AerialMedia.ShuffleBag();
  const catalogue=window.AERIAL_DESKTOP_CATALOGUE_MODE===true;
  const scenes=(window.AERIAL_DESKTOP_PLAYLIST||[]).filter(s=>s&&typeof s.id==='string'&&/^[a-zA-Z0-9_-]+$/.test(s.id)&&(/^videos\/[a-zA-Z0-9_-]+\.webm$/.test(s.file)||(catalogue&&s.streaming===true&&s.file==='http://127.0.0.1:5194/video/'+s.id+'.webm')));
  const ids=scenes.map(s=>s.id),cycleKey='aerial-desktop-cycle-v2';
  let prefs={clockStyle:'glass',clockFormat:'auto',clockDate:true,position:'top',shuffle:true,oledProtection:true},current=null,paused=false,generation=0,timer;
  let lastActivity=Date.now(),epoch=Date.now(),failed=new Set();
  const preview=new URLSearchParams(location.search).has('preview');
  let restored=null,first=true,live=false,loading=false,offset=0,resumeAt=0;
  try{const saved=JSON.parse(localStorage.getItem(cycleKey)||'null');if(saved&&saved.ids===ids.join('|')&&ids.includes(saved.current)&&Array.isArray(saved.remaining)&&saved.remaining.length<=ids.length&&new Set(saved.remaining).size===saved.remaining.length&&!saved.remaining.includes(saved.current)&&saved.remaining.every(id=>ids.includes(id))){restored=saved;prefs.shuffle=saved.shuffle!==false;bag.bag=saved.remaining;bag.last=saved.current;bag.signature=ids.join('|')+prefs.shuffle;}}catch(_){}
  function remember(){if(!current)return;try{localStorage.setItem(cycleKey,JSON.stringify({ids:ids.join('|'),current:current.id,remaining:bag.bag,shuffle:prefs.shuffle,position:Math.min(7200,Math.max(0,paused||loading?resumeAt:offset+(Number(video.currentTime)||0)))}));}catch(_){} }
  if(preview){try{prefs={...prefs,...JSON.parse(localStorage.getItem('aerial-desktop-v1')||'{}')};}catch(_){}$('controls').hidden=false;}
  function text(id,value){if($(id).textContent!==value)$(id).textContent=value;}
  function sanitize(){const safe=AerialCore.clean(prefs);prefs={...prefs,clockStyle:safe.clockStyle,clockFormat:safe.clockFormat,clockDate:safe.clockDate,oledProtection:safe.oledProtection,shuffle:safe.shuffle,position:['top','center','right'].includes(prefs.position)?prefs.position:'top'};}
  function tick(){
    if(paused)return;
    const now=new Date(),clock=AerialCore.clockFace(now,prefs),state=AerialCore.oledState(prefs,Date.now()-lastActivity,Date.now()-epoch);
    text('digits',clock.time);text('period',clock.period);text('date',clock.date);$('period').hidden=!clock.period;$('date').hidden=!prefs.clockDate;
    $('clock').setAttribute('aria-label',clock.label);document.body.dataset.rest=state.rest;
    document.documentElement.style.setProperty('--shift-x',state.x+'px');document.documentElement.style.setProperty('--shift-y',state.y+'px');
    remember();
  }
  function appearance(){sanitize();document.body.dataset.style=prefs.clockStyle;document.body.dataset.position=prefs.position;lastActivity=Date.now();tick();
    $('style-choice').value=prefs.clockStyle;$('format-choice').value=prefs.clockFormat;$('position-choice').value=prefs.position;
    $('date-choice').checked=prefs.clockDate;$('shuffle-choice').checked=prefs.shuffle;$('oled-choice').checked=prefs.oledProtection;
    if(preview)try{localStorage.setItem('aerial-desktop-v1',JSON.stringify(prefs));}catch(_){}
  }
  async function start(scene,position=0){
    clearTimeout(timer);const visit=++generation;resumeAt=position;offset=0;live=false;loading=true;
    if(paused)return;
    let source=scene.local?'videos/'+scene.id+'.webm':scene.file;
    if(scene.streaming&&!scene.local){
      text('status','Preparing the next aerial…');
      try{const response=await fetch('http://127.0.0.1:5194/api/info/'+scene.id);if(!response.ok)throw Error('Media service unavailable');const info=await response.json();if(visit!==generation||paused)return;if(info.busy&&!info.cached){timer=setTimeout(()=>start(scene,position),1500);return;}live=!info.cached;offset=live?position:0;if(live&&position)source+='?t='+position;}
      catch(_){if(visit===generation&&!paused){text('status','Waiting for aerials…');timer=setTimeout(()=>start(scene,position),5000);}return;}
    }
    if(visit!==generation||paused)return;
    video.onloadedmetadata=()=>{if(visit===generation&&!live&&position>0)try{video.currentTime=Number.isFinite(video.duration)?Math.min(position,Math.max(0,video.duration-0.1)):position;}catch(_){} };
    video.src=source;video.muted=true;video.loop=false;text('place',scene.title);text('status',live?'Preparing the next aerial…':'');
    timer=setTimeout(()=>{if(visit===generation){failed.add(scene.id);next();}},live?90000:20000);
    video.play().catch(()=>{if(visit===generation){failed.add(scene.id);next();}});
  }
  function next(){
    clearTimeout(timer);
    if(failed.size>=scenes.length){current=null;generation++;text('status',scenes.length?(catalogue?'No aerial is available right now. Retrying shortly.':'No compatible local aerial could play.'):'Add local aerials with the preparation tool.');video.removeAttribute('src');video.load();if(catalogue)timer=setTimeout(()=>{failed.clear();next();},60000);return;}
    // Keep the catalogue signature stable when a URL fails: changing it would restart the shuffle bag.
    if(first&&restored){current=scenes.find(s=>s.id===restored.current);first=false;start(current,Number.isFinite(restored.position)?Math.min(7200,Math.max(0,restored.position)):0);return;}
    if(prefs.shuffle&&bag.signature&&bag.bag.length===0||!prefs.shuffle&&bag.last===ids[ids.length-1])failed.clear();
    let id;for(let n=0;n<scenes.length;n++){id=bag.next(ids,prefs.shuffle);if(!failed.has(id))break;}
    if(first&&prefs.shuffle&&catalogue){const seed=scenes.find(s=>s.local);if(seed&&seed.id!==id){bag.bag=bag.bag.filter(value=>value!==seed.id);bag.bag.unshift(id);bag.last=seed.id;id=seed.id;}}
    first=false;const scene=scenes.find(s=>s.id===id);if(!scene)return;
    current=scene;resumeAt=0;offset=0;start(scene);remember();
  }
  video.onplaying=()=>{clearTimeout(timer);loading=false;text('status','');remember();};video.onended=next;video.onerror=()=>{if(current&&!paused&&video.getAttribute('src')){failed.add(current.id);next();}};
  function pause(value){if(!!value===paused)return;if(!paused&&!loading)resumeAt=offset+(Number(video.currentTime)||0);paused=!!value;clearTimeout(timer);if(paused){remember();video.pause();if(current?.streaming&&!current.local){generation++;video.removeAttribute('src');video.load();}}else{lastActivity=Date.now();if(current&&!video.getAttribute('src'))start(current,resumeAt);else if(video.getAttribute('src'))video.play().catch(()=>{});tick();}}
  window.wallpaperPropertyListener={
    applyUserProperties(properties){
      const keys={clockstyle:'clockStyle',clockformat:'clockFormat',showdate:'clockDate',clockposition:'position',shuffle:'shuffle',oledcare:'oledProtection'};
      const oldShuffle=prefs.shuffle;for(const [key,target] of Object.entries(keys)){if(properties[key])prefs[target]=properties[key].value;}
      appearance();if(oldShuffle!==prefs.shuffle){bag.bag=[];bag.signature='';remember();}
    },
    setPaused:pause
  };
  document.addEventListener('visibilitychange',()=>pause(document.hidden));
  addEventListener('pagehide',()=>{remember();generation++;video.removeAttribute('src');video.load();});
  addEventListener('mousemove',()=>{lastActivity=Date.now();if(document.body.dataset.rest!=='active')tick();});
  addEventListener('keydown',()=>{lastActivity=Date.now();tick();});
  for(const [id,key,checkbox] of [['style-choice','clockStyle'],['format-choice','clockFormat'],['position-choice','position'],['date-choice','clockDate',true],['shuffle-choice','shuffle',true],['oled-choice','oledProtection',true]]){
    $(id).onchange=()=>{prefs[key]=checkbox?$(id).checked:$(id).value;appearance();};
  }
  $('next').onclick=next;appearance();setInterval(tick,10000);next();
})();
