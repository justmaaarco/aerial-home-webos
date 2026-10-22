(async function () {
  'use strict';
  const {clean,visible,nextIndex}=AerialCore;
  const $=id=>document.getElementById(id);
  let prefs;
  try { prefs=clean(JSON.parse(localStorage.getItem('aerial-home-v1'))); } catch (_) { prefs=clean(); }
  let apps=[],shown=[],focusId=null,panelKind=null,moving=null,saver=false,lastActivity=Date.now(),toastTimer,videoFailed=0;
  let returnFocus=null,videoSuspended=false;
  const scenes=[
    {title:'Hong Kong',url:'http://sylvan.apple.com/Aerials/2x/Videos/HK_H004_C013_4K_SDR_HEVC.mov'},
    {title:'London',url:'http://sylvan.apple.com/Aerials/2x/Videos/comp_L004_C011_PS_v01_SDR_PS_20180925_SDR_4K_HEVC.mov'},
    {title:'New York City',url:'http://sylvan.apple.com/Aerials/2x/Videos/comp_N013_C004_PS_v01_SDR_PS_20180925_F1970F7193_SDR_4K_HEVC.mov'}
  ];
  const paths={
    search:'<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>',
    controls:'<path d="M4 7h16M4 17h16"/><circle cx="9" cy="7" r="3" fill="currentColor"/><circle cx="16" cy="17" r="3" fill="currentColor"/>',
    tv:'<rect x="2" y="5" width="20" height="14" rx="4"/><path d="m9 10 6 3-6 3z" fill="currentColor" stroke="none"/>',
    music:'<path d="M10 17V5l10-2v12M10 8l10-2"/><ellipse cx="6.5" cy="18" rx="3.5" ry="2.5" fill="currentColor"/><ellipse cx="16.5" cy="16" rx="3.5" ry="2.5" fill="currentColor"/>',
    photos:'<circle cx="12" cy="6.5" r="4" fill="#f6b42c"/><circle cx="17" cy="9.5" r="4" fill="#e65f82"/><circle cx="16" cy="16" r="4" fill="#8f74db"/><circle cx="8" cy="16" r="4" fill="#54b0db"/><circle cx="7" cy="9" r="4" fill="#7ac174"/>',
    play:'<path d="m9 5 12 7-12 7z" fill="currentColor" stroke="none"/>',
    game:'<path d="M8 8h8c3 0 4 2 5 7s-2 7-5 2H8c-3 5-6 3-5-2S5 8 8 8Z"/><path d="M8 10v6m-3-3h6m5-2h.01m2 3h.01"/>',
    settings:'<path d="m9 3-1 3-3 1 1 3-2 2 2 3-1 3 3 1 1 3h6l1-3 3-1-1-3 2-3-2-2 1-3-3-1-1-3Z"/><circle cx="12" cy="12" r="4"/>',
    audio:'<path d="M5 15v-3a7 7 0 0 1 14 0v3M5 13H3v6h4v-6Zm14 0h2v6h-4v-6Z"/>',
    sport:'<circle cx="12" cy="12" r="9"/><path d="m12 7 5 4-2 6H9l-2-6Zm0-4v4M3 10l4 1m10 0 4-1M7 20l2-3m6 0 2 3"/>',
    input:'<rect x="3" y="4" width="18" height="14" rx="3"/><path d="M9 21h6m-3-3v3m-6-9h10m-4-4 4 4-4 4"/>',
    globe:'<circle cx="12" cy="12" r="9"/><ellipse cx="12" cy="12" rx="4" ry="9"/><path d="M3 12h18M5 6h14M5 18h14"/>',
    moon:'<path d="M20 15A9 9 0 0 1 9 4a9 9 0 1 0 11 11Z"/>',
    volume:'<path d="M4 9h4l5-5v16l-5-5H4Zm12-1a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14"/>',
    person:'<circle cx="12" cy="8" r="4"/><path d="M4 22v-3a8 8 0 0 1 16 0v3"/>',
    folder:'<path d="M3 6h7l2 3h9v11H3Z"/>',
    picture:'<rect x="3" y="3" width="18" height="18" rx="4"/><path d="m3 17 6-6 4 4 3-3 5 5"/><circle cx="16" cy="8" r="1"/>',
    check:'<path d="m4 12 5 5L20 6"/>'
  };
  function svg(name) { return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'+(paths[name]||paths.play)+'</svg>'; }
  function el(tag,className,text) { const n=document.createElement(tag); if(className)n.className=className; if(text!==undefined)n.textContent=text; return n; }
  function toast(message) { $('toast').textContent=message;$('toast').hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(()=>$('toast').hidden=true,4500); }
  async function safely(action) { try { await action(); } catch(error) { toast(error.message || 'The TV could not complete that action'); } }
  function save() {
    try { localStorage.setItem('aerial-home-v1',JSON.stringify(prefs)); } catch (_) { toast('Settings could not be saved: TV storage is full'); }
    applyAppearance();
  }
  function applyAppearance() {
    const hour=new Date().getHours();
    document.body.classList.toggle('light',prefs.appearance==='light' || (prefs.appearance==='auto' && hour>=7 && hour<19));
    document.body.classList.toggle('reduced',prefs.reduceMotion);
    document.documentElement.style.setProperty('--cols',prefs.columns);
    $('profile-name').textContent=prefs.profile;
  }
  function fit() { $('stage').style.transform='scale('+Math.min(innerWidth/1920,innerHeight/1080)+')'; }
  addEventListener('resize',fit);fit();applyAppearance();
  $('search-button').innerHTML=svg('search');$('controls-button').innerHTML=svg('controls');
  $('search-button').onclick=search;$('controls-button').onclick=controls;
  function clock() {
    const text=new Date().toLocaleTimeString([], {hour:'numeric',minute:'2-digit'});
    $('clock').textContent=text;$('saver-clock').textContent=text;
  }
  clock();setInterval(clock,10000);
  function tile(app) {
    const b=el('button','app');b.dataset.id=app.id;b.setAttribute('aria-label',app.title);
    const face=el('span','tile'+(app.folder?' folder-tile':''));face.style.setProperty('--tile',app.color||'#ffffff25');
    if(app.folder) {
      app.apps.slice(0,6).forEach(id=>{const a=apps.find(a=>a.id===id);if(a){const d=el('span','folder-dot',a.title[0]);face.append(d);}});
    } else if(app.image) {
      const img=el('img');img.src=app.image;img.alt='';img.onerror=()=>{img.remove();face.innerHTML=svg(app.glyph);};face.append(img);
    } else face.innerHTML=svg(app.glyph);
    b.append(face,el('span','app-label',app.title));
    b.onfocus=()=>{focusId=app.id;};
    b.onclick=()=>{if(moving){moving=null;moveOriginal=null;save();render(app.id);$('hint').textContent='↑ Control Center';return;}safely(()=>openApp(app));};
    b.oncontextmenu=e=>{e.preventDefault();appMenu(app);};
    let hold;
    b.onpointerdown=()=>{hold=setTimeout(()=>appMenu(app),850);};
    b.onpointerup=b.onpointercancel=b.onpointerleave=()=>clearTimeout(hold);
    return b;
  }
  function render(id=focusId,focus=true) {
    const wasGrid=document.activeElement?.classList.contains('app') && !panelKind;
    shown=visible(apps,prefs);
    const grid=$('grid');grid.replaceChildren(...shown.map(tile));
    $('empty').hidden=shown.length>0;
    if(moving) Array.from(grid.children).find(b=>b.dataset.id===moving)?.classList.add('moving');
    if(focus || wasGrid) (Array.from(grid.children).find(b=>b.dataset.id===id) || grid.firstElementChild)?.focus({preventScroll:true});
  }
  async function openApp(app) {
    if(app.folder) { folder(app);return; }
    if(app.id==='settings') { settings();return; }
    if(!TV.live) { toast('Preview: '+app.title+' opens on your LG TV');return; }
    await safely(async()=>{await TV.launch(app.id);});
  }
  function panel(title,kind='generic',wide=false) {
    moving=null;moveOriginal=null;returnFocus=document.activeElement?.dataset?.id || focusId;
    const p=$('panel');p.replaceChildren();p.hidden=false;p.classList.toggle('wide',wide);panelKind=kind;
    document.body.classList.add('panel-open');$('home').setAttribute('inert','');$('toolbar').setAttribute('inert','');
    const header=el('div','panel-header'),h=el('h1','',title);h.id='panel-title';
    const close=el('button','close','×');close.setAttribute('aria-label','Close');close.onclick=closePanel;
    header.append(h,close);p.append(header);queueMicrotask(()=>(p.querySelector('input')||p.querySelector('.control')||p.querySelector('.app')||close).focus());return p;
  }
  function closePanel() {
    $('panel').hidden=true;panelKind=null;document.body.classList.remove('panel-open');
    $('home').removeAttribute('inert');$('toolbar').removeAttribute('inert');render(returnFocus);
  }
  function section(p,text) { p.append(el('div','panel-section',text)); }
  function note(p,text) { p.append(el('p','panel-note',text)); }
  function control(p,label,icon,detail,action) {
    const b=el('button','control');if(icon)b.innerHTML=svg(icon);b.append(el('span','',label));
    if(detail!==undefined)b.append(el('small','',detail));b.onclick=()=>safely(action);p.append(b);return b;
  }
  async function controls() {
    const p=panel('Control Center','controls');
    control(p,prefs.profile,'person','Profile',profile);
    section(p,'TV');
    const audio=control(p,'Sound','volume',TV.live?'Loading…':'Preview',sound);
    if(TV.live) TV.call('luna://com.webos.audio/getVolume').then(r=>{if(audio.isConnected)audio.querySelector('small').textContent=(r.volume??r.volumeStatus?.volume??'')+' · '+((r.muteStatus??r.volumeStatus?.muteStatus)?'Muted':'TV speakers');}).catch(()=>{if(audio.isConnected)audio.querySelector('small').textContent='Open controls';});
    control(p,'Sleep timer','moon','Off / 15 / 30 / 60 / 90 min',sleep);
    control(p,'Inputs','input','HDMI',inputs);
    section(p,'PERSONALIZE');
    control(p,'Aerials','picture',scenes[prefs.scene].title,aerialSettings);
    control(p,'Screen saver','moon','Start now',()=>{closePanel();startSaver();});
    control(p,'Settings','settings','',settings);
  }
  async function sound() {
    const p=panel('Sound','sound');let volume=20,muted=false;
    if(TV.live) {
      try { const r=await TV.call('luna://com.webos.audio/getVolume');volume=r.volume??r.volumeStatus?.volume??20;muted=r.muteStatus??r.volumeStatus?.muteStatus??false; }
      catch(error) { note(p,error.message); }
    }
    if(panelKind!=='sound')return;
    const label=el('p','panel-note','Volume '+volume);p.append(label);
    async function setVolume(delta) { const target=Math.max(0,Math.min(100,volume+delta));if(TV.live)await TV.call('luna://com.webos.audio/setVolume',{volume:target});volume=target;label.textContent='Volume '+volume; }
    control(p,'Volume up','volume','+5',()=>setVolume(5));control(p,'Volume down','volume','−5',()=>setVolume(-5));
    const mute=control(p,muted?'Unmute':'Mute','volume','',async()=>{if(TV.live)await TV.call('luna://com.webos.audio/setMuted',{muted:!muted});muted=!muted;mute.querySelector('span').textContent=muted?'Unmute':'Mute';});
    note(p,TV.live?'Changes the TV volume. External audio systems may control their own volume.':'Browser preview: volume changes stay in this preview.');
  }
  function sleep() {
    const p=panel('Sleep timer','sleep');
    [0,15,30,60,90].forEach(minutes=>control(p,minutes?minutes+' minutes':'Turn off timer','moon','',async()=>{
      if(TV.live) await TV.call('luna://com.webos.service.tvReservationAgent/timer/sleep/add',{sleepTimeMinute:minutes});
      toast(minutes?'Sleep timer set for '+minutes+' minutes':'Sleep timer cancelled');closePanel();
    }));
    note(p,'Uses the TV sleep timer; it continues after leaving Aerial Home.');
  }
  function inputs() {
    const p=panel('Inputs','inputs');const list=apps.filter(a=>a.input);
    list.forEach(a=>control(p,a.title,'input','',()=>openApp(a)));
    if(!list.length)note(p,'No HDMI launch points were reported by the TV.');
  }
  function profile() {
    const p=panel('Profile','profile');const input=el('input');input.type='text';input.maxLength=32;input.value=prefs.profile;input.setAttribute('aria-label','Profile name');p.append(input);
    control(p,'Save name','check','',()=>{prefs.profile=input.value.trim()||'Home';save();controls();});
    note(p,'A local home-screen name. Streaming services keep their own accounts and profiles.');
  }
  function settings() {
    const p=panel('Settings','settings');
    section(p,'APPEARANCE');
    control(p,'Appearance','picture',prefs.appearance,()=>{prefs.appearance={dark:'light',light:'auto',auto:'dark'}[prefs.appearance];save();settings();});
    control(p,'App size','tv',prefs.columns===6?'Standard · 6 columns':'Large · 5 columns',()=>{prefs.columns=prefs.columns===6?5:6;save();render(null,false);settings();});
    control(p,'Reduce motion','controls',prefs.reduceMotion?'On':'Off',()=>{prefs.reduceMotion=!prefs.reduceMotion;save();settings();});
    control(p,'Aerials & screen saver','picture','4K HEVC',aerialSettings);
    section(p,'HOME SCREEN');
    control(p,'New folder','folder','',createFolder);
    control(p,'Hidden apps','tv',String(prefs.hidden.length),hiddenApps);
    control(p,'Profile name','person',prefs.profile,profile);
    control(p,'LG TV settings','settings','',()=>TV.live?TV.launch('com.palm.app.settings'):toast('Preview: opens LG TV settings'));
    section(p,'ABOUT');note(p,'Aerial Home 0.1.0 · Independent open-source launcher. Built for webOS. Not affiliated with Apple or LG.');
    note(p,TV.live?'Connected to LG webOS. Homebrew Channel supplies app discovery and local artwork.':'Browser preview — sample tiles are original placeholder artwork. Real installed apps appear on the TV.');
  }
  function aerialSettings() {
    const p=panel('Aerials','aerials');
    control(p,'Video background','picture',prefs.aerial?'On':'Off',()=>{prefs.aerial=!prefs.aerial;save();playScene();aerialSettings();});
    section(p,'CITYSCAPES · 3840 × 2160');
    scenes.forEach((s,i)=>control(p,s.title,'picture',prefs.scene===i?'Selected':'4K · HEVC SDR',()=>{prefs.scene=i;prefs.aerial=true;videoFailed=0;save();playScene();aerialSettings();}));
    section(p,'SCREEN SAVER');
    control(p,'Start after','moon',prefs.saverMinutes?prefs.saverMinutes+' minutes':'Never',()=>{const choices=[0,1,3,5,10];prefs.saverMinutes=choices[(choices.indexOf(prefs.saverMinutes)+1)%choices.length];save();aerialSettings();});
    control(p,'Start screen saver','moon','',()=>{closePanel();startSaver();});
    note(p,'Apple-hosted videos stream directly from their public HTTP media endpoints. No video files are bundled. Aerial availability can change.');
    if(!TV.live)note(p,'Most desktop browsers cannot decode these HEVC MOV streams. The TV uses its hardware video decoder.');
  }
  function search() {
    const p=panel('Search apps','search',true);const input=el('input');input.type='search';input.placeholder='Find an app or input';input.setAttribute('aria-label','Search apps');
    const results=el('div','panel-results');p.append(input,results);
    function update() { results.replaceChildren();const matches=visible(apps,prefs,input.value);matches.forEach(a=>control(results,a.title,a.glyph || 'folder',a.input?'Input':'',()=>openApp(a)));if(!matches.length)note(results,'No matching apps'); }
    input.oninput=update;update();
  }
  function createFolder() {
    const p=panel('New folder','new-folder');const input=el('input');input.type='text';input.placeholder='Folder name';input.maxLength=32;input.setAttribute('aria-label','Folder name');p.append(input);
    control(p,'Create folder','folder','',()=>{const title=input.value.trim();if(!title){toast('Enter a folder name');input.focus();return;}const f={id:'folder-'+Date.now().toString(36),title,apps:[]};prefs.folders.push(f);save();folderMembers(f);});
  }
  function folderMembers(f) {
    const p=panel(f.title,'folder-members');note(p,'Choose the apps to include. Select Done when finished.');
    control(p,'Done','check','',()=>{save();closePanel();});
    apps.filter(a=>a.id!=='settings').forEach(a=>{
      const b=control(p,a.title,a.glyph,f.apps.includes(a.id)?'✓':'',()=>{
        if(f.apps.includes(a.id))f.apps=f.apps.filter(id=>id!==a.id);
        else { prefs.folders.forEach(other=>{if(other.id!==f.id)other.apps=other.apps.filter(id=>id!==a.id);});f.apps.push(a.id);prefs.hidden=prefs.hidden.filter(id=>id!==a.id); }
        b.querySelector('small').textContent=f.apps.includes(a.id)?'✓':'';save();
      });
    });
  }
  function folder(f) {
    const p=panel(f.title,'folder',true),grid=el('div');grid.id='folder-grid';grid.append(...visible(apps,prefs,'',f.id).map(tile));p.append(grid);
    control(p,'Edit folder','folder','',()=>folderMembers(f));
    control(p,'Remove folder','folder','Keep apps on Home',()=>{prefs.folders=prefs.folders.filter(x=>x.id!==f.id);save();closePanel();});
  }
  function hiddenApps() {
    const p=panel('Hidden apps','hidden');const hidden=apps.filter(a=>prefs.hidden.includes(a.id));
    hidden.forEach(a=>control(p,a.title,a.glyph,'Show',()=>{prefs.hidden=prefs.hidden.filter(id=>id!==a.id);save();hiddenApps();}));if(!hidden.length)note(p,'No apps are hidden. Hold OK on an app to hide it.');
  }
  function appMenu(a) {
    const p=panel(a.title,'app-menu');
    control(p,'Open',a.glyph || 'folder','',()=>openApp(a));
    control(p,'Move','controls','Use arrows, then OK',()=>{closePanel();moving=a.id;render(a.id);$('hint').textContent='Arrows to move · OK to finish · Back to cancel';});
    if(!a.folder && a.id!=='settings')control(p,'Hide app','tv','',()=>{prefs.hidden.push(a.id);save();closePanel();});
    if(a.folder)control(p,'Edit folder','folder','',()=>folderMembers(a));
    else if(prefs.folders.length) prefs.folders.forEach(f=>control(p,'Add to '+f.title,'folder','',()=>{prefs.folders.forEach(o=>o.apps=o.apps.filter(id=>id!==a.id));f.apps.push(a.id);save();closePanel();}));
  }
  const video=$('aerial');
  function stopVideo() { video.pause();video.removeAttribute('src');video.load(); }
  function playScene() {
    $('scene-title').textContent=scenes[prefs.scene].title;$('saver-place').textContent=scenes[prefs.scene].title;
    if(!prefs.aerial || document.hidden || !TV.live) { stopVideo();return; }
    video.src=scenes[prefs.scene].url;video.muted=true;
    video.play().catch(()=>{toast('Aerial could not start. You can retry in Settings → Aerials.');});
  }
  video.onended=()=>{prefs.scene=(prefs.scene+1)%scenes.length;playScene();};
  video.onerror=()=>{
    if(!video.getAttribute('src'))return;
    if(++videoFailed<scenes.length){prefs.scene=(prefs.scene+1)%scenes.length;playScene();}
    else {stopVideo();toast('Aerial videos are unavailable. Check the TV network, then retry in Settings.');}
  };
  video.onplaying=()=>{videoFailed=0;};
  document.addEventListener('visibilitychange',()=>{if(document.hidden){videoSuspended=true;stopVideo();}else{lastActivity=Date.now();if(videoSuspended){videoSuspended=false;playScene();}clock();}});
  function startSaver() { if(!prefs.aerial){toast('Enable video background in Aerials first');return;}saver=true;document.body.classList.add('saver');$('saver-caption').hidden=false;$('home').setAttribute('inert','');$('toolbar').setAttribute('inert','');document.activeElement?.blur(); }
  function activity() {
    lastActivity=Date.now();
    if(saver){saver=false;document.body.classList.remove('saver');$('saver-caption').hidden=true;$('home').removeAttribute('inert');$('toolbar').removeAttribute('inert');render();return true;}
    return false;
  }
  addEventListener('pointermove',()=>activity());addEventListener('pointerdown',e=>{if(activity()){e.preventDefault();e.stopImmediatePropagation();}},true);
  setInterval(()=>{if(!document.hidden&&!panelKind&&!saver&&prefs.saverMinutes&&Date.now()-lastActivity>prefs.saverMinutes*60000)startSaver();},5000);
  let heldKey=null,holdTimer,longPress=false,moveOriginal=null;
  addEventListener('keyup',e=>{
    if(e.key==='Enter' && heldKey==='Enter'){clearTimeout(holdTimer);heldKey=null;if(!longPress){e.preventDefault();document.activeElement?.click();}longPress=false;}
  },true);
  addEventListener('keydown',e=>{
    if(activity()){e.preventDefault();return;}
    const back=e.key==='Escape'||e.keyCode===461||e.key==='Backspace'&&!/INPUT/.test(document.activeElement?.tagName);
    if(back) {e.preventDefault();if(moving){moving=null;if(moveOriginal){prefs.order=moveOriginal;save();moveOriginal=null;}render();$('hint').textContent='↑ Control Center';}else if(panelKind)closePanel();else controls();return;}
    if(e.key==='Enter' && !/INPUT/.test(document.activeElement?.tagName)) {
      e.preventDefault();if(heldKey || e.repeat)return;heldKey='Enter';longPress=false;
      const id=document.activeElement?.dataset?.id;
      if(id&&!panelKind&&!moving)holdTimer=setTimeout(()=>{longPress=true;appMenu(apps.find(a=>a.id===id)||prefs.folders.find(f=>f.id===id));},850);
      return;
    }
    if((e.key.toLowerCase()==='c'||e.keyCode===403)&&!/INPUT/.test(document.activeElement?.tagName)){e.preventDefault();controls();return;}
    if(e.key.toLowerCase()==='s'&&!/INPUT/.test(document.activeElement?.tagName)){e.preventDefault();search();return;}
    if(e.key==='Tab'&&panelKind){const all=Array.from($('panel').querySelectorAll('button:not(:disabled),input'));let i=all.indexOf(document.activeElement);i=(i+(e.shiftKey?-1:1)+all.length)%all.length;e.preventDefault();all[i]?.focus();return;}
    if(!e.key.startsWith('Arrow'))return;
    if(document.activeElement?.tagName==='INPUT' && ['ArrowLeft','ArrowRight'].includes(e.key))return;
    e.preventDefault();const dir=e.key.slice(5).toLowerCase();
    if(panelKind) {
      const all=Array.from($('panel').querySelectorAll('button:not(:disabled),input'));
      const i=all.indexOf(document.activeElement),delta=dir==='down'||dir==='right'?1:-1;
      all[Math.max(0,Math.min(all.length-1,i+delta))]?.focus();return;
    }
    const current=Array.from($('grid').children),index=current.indexOf(document.activeElement);
    if(index<0){if(dir==='down')current[0]?.focus();else if(dir==='left')$('search-button').focus();else if(dir==='right')$('controls-button').focus();return;}
    const next=nextIndex(index,dir,current.length,prefs.columns);
    if(next<0){if(!moving)controls();return;}
    if(moving&&next!==index) {
      if(!moveOriginal)moveOriginal=prefs.order.slice();
      const order=shown.map(a=>a.id),[id]=order.splice(index,1);order.splice(next,0,id);prefs.order=order;
      render(id);save();return;
    }
    current[next]?.focus();current[next]?.scrollIntoView({block:'nearest',behavior:prefs.reduceMotion?'auto':'smooth'});
  },true);
  try { apps=await TV.listApps();render(); }
  catch(error) { apps=[{id:'settings',title:'Settings',glyph:'settings',color:'#aab2bd'}];render();toast('App discovery failed: '+error.message); }
  $('hero-greeting').textContent=TV.live?'Your world. Ready to play.':'Your evening, beautifully simple.';
  $('hero-subtitle').textContent=TV.live?'Apps, inputs and a little room to unwind.':'Everything you love. One place.';
  playScene();
  function routeLaunch(event) {
    try {
      const params=event?.detail || JSON.parse(window.PalmSystem?.launchParams || '{}');
      const routes={controls,settings,inputs,search};
      if(Object.hasOwn(routes,params.screen))routes[params.screen]();
    } catch (_) { /* Missing launch params simply open Home. */ }
  }
  document.addEventListener('webOSRelaunch',routeLaunch,true);routeLaunch();
  // Bounded concurrency avoids flooding the privileged bridge during startup.
  const queue=apps.filter(a=>a.icon);
  await Promise.all([0,1,2].map(async()=>{while(queue.length){const a=queue.shift();try {a.image=await TV.icon(a);}catch(_) { /* Keep the original fallback icon. */ }if(a.image&&!panelKind)render(focusId,false);}}));
})();
