(function(){
  'use strict';
  const $=id=>document.getElementById(id),video=$('aerial'),bag=new AerialMedia.ShuffleBag();
  const scenes=(window.AERIAL_DESKTOP_PLAYLIST||[]).filter(s=>s&&typeof s.id==='string'&&/^videos\/[a-zA-Z0-9_-]+\.webm$/.test(s.file));
  let prefs={clockStyle:'glass',clockFormat:'auto',clockDate:true,position:'top',shuffle:true,oledProtection:true},current=null,paused=false,generation=0,timer;
  let lastActivity=Date.now(),epoch=Date.now(),failed=new Set();
  const preview=new URLSearchParams(location.search).has('preview');
  if(preview){try{prefs={...prefs,...JSON.parse(localStorage.getItem('aerial-desktop-v1')||'{}')};}catch(_){}$('controls').hidden=false;}
  function text(id,value){if($(id).textContent!==value)$(id).textContent=value;}
  function sanitize(){const safe=AerialCore.clean(prefs);prefs={...prefs,clockStyle:safe.clockStyle,clockFormat:safe.clockFormat,clockDate:safe.clockDate,oledProtection:safe.oledProtection,shuffle:safe.shuffle,position:['top','center','right'].includes(prefs.position)?prefs.position:'top'};}
  function tick(){
    if(paused)return;
    const now=new Date(),clock=AerialCore.clockFace(now,prefs),state=AerialCore.oledState(prefs,Date.now()-lastActivity,Date.now()-epoch);
    text('digits',clock.time);text('period',clock.period);text('date',clock.date);$('period').hidden=!clock.period;$('date').hidden=!prefs.clockDate;
    $('clock').setAttribute('aria-label',clock.label);document.body.dataset.rest=state.rest;
    document.documentElement.style.setProperty('--shift-x',state.x+'px');document.documentElement.style.setProperty('--shift-y',state.y+'px');
  }
  function appearance(){sanitize();document.body.dataset.style=prefs.clockStyle;document.body.dataset.position=prefs.position;lastActivity=Date.now();tick();
    $('style-choice').value=prefs.clockStyle;$('format-choice').value=prefs.clockFormat;$('position-choice').value=prefs.position;
    $('date-choice').checked=prefs.clockDate;$('shuffle-choice').checked=prefs.shuffle;$('oled-choice').checked=prefs.oledProtection;
    if(preview)try{localStorage.setItem('aerial-desktop-v1',JSON.stringify(prefs));}catch(_){}
  }
  function next(){
    clearTimeout(timer);const available=scenes.filter(s=>!failed.has(s.id));
    if(!available.length){current=null;generation++;text('status',scenes.length?'No compatible local aerial could play.':'Add local aerials with the preparation tool.');video.removeAttribute('src');video.load();return;}
    const id=bag.next(available.map(s=>s.id),prefs.shuffle),scene=available.find(s=>s.id===id);if(!scene)return;
    current=scene;const visit=++generation;video.src=scene.file;video.muted=true;video.loop=false;
    text('place',scene.title);text('status','');
    timer=setTimeout(()=>{if(visit===generation){failed.add(scene.id);next();}},20000);
    if(!paused)video.play().catch(()=>{if(visit===generation){failed.add(scene.id);next();}});
  }
  video.onplaying=()=>{clearTimeout(timer);text('status','');};video.onended=next;video.onerror=()=>{if(current){failed.add(current.id);next();}};
  function pause(value){paused=!!value;clearTimeout(timer);if(paused)video.pause();else{lastActivity=Date.now();tick();if(video.getAttribute('src'))video.play().catch(()=>{});}}
  window.wallpaperPropertyListener={
    applyUserProperties(properties){
      const keys={clockstyle:'clockStyle',clockformat:'clockFormat',showdate:'clockDate',clockposition:'position',shuffle:'shuffle',oledcare:'oledProtection'};
      const oldShuffle=prefs.shuffle;for(const [key,target] of Object.entries(keys)){if(properties[key])prefs[target]=properties[key].value;}
      appearance();if(oldShuffle!==prefs.shuffle)bag.bag=[];
    },
    setPaused:pause
  };
  document.addEventListener('visibilitychange',()=>pause(document.hidden));
  addEventListener('mousemove',()=>{lastActivity=Date.now();if(document.body.dataset.rest!=='active')tick();});
  addEventListener('keydown',()=>{lastActivity=Date.now();tick();});
  for(const [id,key,checkbox] of [['style-choice','clockStyle'],['format-choice','clockFormat'],['position-choice','position'],['date-choice','clockDate',true],['shuffle-choice','shuffle',true],['oled-choice','oledProtection',true]]){
    $(id).onchange=()=>{prefs[key]=checkbox?$(id).checked:$(id).value;appearance();};
  }
  $('next').onclick=next;appearance();setInterval(tick,10000);next();
})();
