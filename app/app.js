(async function () {
  'use strict';
  const {clean,visible,nextIndex,homeApps,starterFavorites,greeting}=AerialCore;
  const {ShuffleBag,variants:mediaVariants,label:qualityLabel}=AerialMedia;
  const $=id=>document.getElementById(id);
  let prefs;
  try { prefs=clean(JSON.parse(localStorage.getItem('aerial-home-v1'))); } catch (_) { prefs=clean(); }
  let apps=[],shown=[],focusId=null,panelKind=null,moving=null,saver=false,lastActivity=Date.now(),toastTimer,videoFailed=0;
  let returnFocus=null,videoSuspended=false,homeDirty=false,greetingKey='',lastGreetingIndex=-1;
  let caps={dolbyVision:false,hdr10:false,uhd:true};
  let startupStorage,artworkDatabase;
  try{startupStorage=window.localStorage;}catch(_){}
  try{artworkDatabase=window.indexedDB;}catch(_){}
  const cache=AerialCache.create(startupStorage,artworkDatabase),cached=TV.live?cache.read():null;
  const startup=window.AerialStartup={scriptMs:Math.round(performance.now()),cachedHome:!!cached?.apps.length};
  function startupMark(key){if(startup[key]===undefined)startup[key]=Math.round(performance.now());}
  function startupPaint(key){if(startup[key]===undefined)requestAnimationFrame(()=>requestAnimationFrame(()=>startupMark(key)));}
  let mediaReady=false,pendingSceneId=null,inputsLoading=false;
  let inputRowOpen=false,inputReturnFocus=null;
  let oledDimmed=false,wakeKey=null,suppressWakeClick=false;
  if(cached?.caps)caps=cached.caps;
  function persistStartup(){if(TV.live)cache.save(apps,caps);}
  const scenes=APPLE_AERIALS.scenes,bag=new ShuffleBag(),sceneMap=new Map(scenes.map(s=>[s.id,s]));
  let currentScene=null,currentVariant=null,playGeneration=0,startTimer,playRequest=false,preparingTitle='';
  const failedVariants=new Set(),failedScenes=new Set();
  document.body.classList.toggle('tv',TV.live);
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
  paths.apps='<rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="7" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/><rect x="14" y="14" width="7" height="7" rx="2"/>';
  function svg(name) { return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'+(paths[name]||paths.play)+'</svg>'; }
  function el(tag,className,text) { const n=document.createElement(tag); if(className)n.className=className; if(text!==undefined)n.textContent=text; return n; }
  function toast(message) { $('toast').textContent=message;$('toast').hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(()=>$('toast').hidden=true,4500); }
  async function safely(action) { try { await action(); } catch(error) { toast(error.message || 'The TV could not complete that action'); } }
  function save() {
    try { localStorage.setItem('aerial-home-v1',JSON.stringify(prefs)); } catch (_) { toast('Settings could not be saved: TV storage is full'); }
    homeDirty=true;applyAppearance();
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
  $('search-button').innerHTML=svg('search');$('controls-button').innerHTML=svg('controls');$('apps-button').innerHTML=svg('apps');
  $('search-button').onclick=search;$('controls-button').onclick=controls;$('apps-button').onclick=allApps;
  function clock(refreshGreeting=false) {
    const date=new Date(),g=greeting(date.getHours(),Math.random,lastGreetingIndex),key=date.toDateString()+g.period;
    if(refreshGreeting||key!==greetingKey){greetingKey=key;lastGreetingIndex=g.index;$('hero-greeting').textContent=g.title;$('hero-subtitle').textContent=g.phrase;}
    const text=new Date().toLocaleTimeString([], {hour:'numeric',minute:'2-digit'});
    $('clock').textContent=text;$('saver-clock').textContent=text;
  }
  clock();setInterval(()=>clock(),10000);
  function tile(app) {
    const b=el('button','app');b.dataset.id=app.id;b.setAttribute('aria-label',app.title);
    const face=el('span','tile'+(app.folder?' folder-tile':''));face.style.setProperty('--tile',app.color||'#ffffff25');
    if(app.folder) {
      app.apps.slice(0,6).forEach(id=>{const a=apps.find(a=>a.id===id);if(a){const d=el('span','folder-dot',a.title[0]);face.append(d);}});
    } else if(app.image) {
      const img=el('img');img.onload=()=>startupPaint('firstIconMs');img.src=app.image;img.alt='';img.onerror=()=>{img.remove();face.innerHTML=svg(app.glyph);};face.append(img);
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
    shown=homeApps(apps,prefs).filter(a=>!a.input);homeDirty=false;
    const grid=$('grid');grid.replaceChildren(...shown.map(tile));
    if(shown.length){startupMark('homeDomMs');startupPaint('homePaintMs');}
    $('empty').hidden=shown.length>0;
    if(moving) Array.from(grid.children).find(b=>b.dataset.id===moving)?.classList.add('moving');
    if(focus || wasGrid) (Array.from(grid.children).find(b=>b.dataset.id===id) || grid.firstElementChild)?.focus({preventScroll:true});
    hydrateIcons(shown);
    if(inputRowOpen)renderInputRow();
  }
  function renderInputRow(focus=false,column=0) {
    const row=$('input-row');row.hidden=!inputRowOpen;if(!inputRowOpen)return;
    const selected=document.activeElement?.classList.contains('input-button')?document.activeElement.dataset.id:null;
    const list=apps.filter(a=>a.input);
    row.replaceChildren(...list.map(a=>{
      const b=el('button','input-button');b.dataset.id=a.id;b.setAttribute('aria-label',a.title);
      const face=el('span','tile');face.innerHTML=svg('input');b.append(face,el('span','input-label',a.title));
      b.onclick=()=>safely(()=>openApp(a));return b;
    }));
    if(!list.length)row.append(el('span','panel-note',inputsLoading?'Loading inputs…':'No inputs available'));
    if(focus||selected){const buttons=Array.from(row.children).filter(b=>b.tagName==='BUTTON');(buttons.find(b=>b.dataset.id===selected)||buttons[Math.min(column,buttons.length-1)])?.focus({preventScroll:true});}
    row.scrollIntoView({block:'nearest',behavior:'auto'});
  }
  function showInputRow(column=0) {
    inputReturnFocus=focusId;inputRowOpen=true;document.body.classList.add('inputs-open');renderInputRow(true,column);
  }
  function hideInputRow(focus=true) {
    if(!inputRowOpen)return;inputRowOpen=false;$('input-row').hidden=true;document.body.classList.remove('inputs-open');
    if(focus)(Array.from($('grid').children).find(b=>b.dataset.id===inputReturnFocus)||$('grid').firstElementChild||$('apps-button')).focus({preventScroll:true});
  }
  async function openApp(app) {
    if(app.folder) { folder(app);return; }
    if(app.id==='settings') { settings();return; }
    if(!TV.live) { toast('Preview: '+app.title+' opens on your LG TV');return; }
    await safely(async()=>{await TV.launch(app.id);const u=prefs.usage[app.id]||{count:0,last:0};prefs.usage[app.id]={count:u.count+1,last:Date.now()};save();});
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
    $('home').removeAttribute('inert');$('toolbar').removeAttribute('inert');
    if(homeDirty)render(returnFocus);else(Array.from($('grid').children).find(b=>b.dataset.id===returnFocus)||$('grid').firstElementChild||$('apps-button')).focus({preventScroll:true});
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
    control(p,'All apps','apps','',allApps);
    section(p,'TV');
    const audio=control(p,'Sound','volume',TV.live?'Loading…':'Preview',sound);
    if(TV.live) TV.call('luna://com.webos.audio/getVolume').then(r=>{if(audio.isConnected)audio.querySelector('small').textContent=(r.volume??r.volumeStatus?.volume??'')+' · '+((r.muteStatus??r.volumeStatus?.muteStatus)?'Muted':'TV speakers');}).catch(()=>{if(audio.isConnected)audio.querySelector('small').textContent='Open controls';});
    control(p,'Sleep timer','moon','Off / 15 / 30 / 60 / 90 min',sleep);
    control(p,'Inputs','input','HDMI',inputs);
    section(p,'PERSONALIZE');
    control(p,'Aerials','picture',currentScene?.title || 'Apple catalogue',aerialSettings);
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
    if(!list.length)note(p,inputsLoading?'Loading HDMI inputs…':'No HDMI launch points were reported by the TV.');
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
    control(p,'OLED dimming','moon',prefs.oledDimming?'After 30 minutes idle':'Off',oledSettings);
    section(p,'HOME SCREEN');
    control(p,'Apps on Home','apps',{'most-used':'Most used','favorites':'Favourites','all':'All apps'}[prefs.homeMode],homeSelection);
    control(p,'New folder','folder','',createFolder);
    control(p,'Hidden apps','tv',String(prefs.hidden.length),hiddenApps);
    control(p,'Profile name','person',prefs.profile,profile);
    control(p,'LG TV settings','settings','',()=>TV.live?TV.launch('com.palm.app.settings'):toast('Preview: opens LG TV settings'));
    section(p,'ABOUT');note(p,'Aerial Home 0.2.3 · Independent open-source launcher. Built for webOS. Not affiliated with Apple or LG.');
    note(p,TV.live?'Connected to LG webOS. Homebrew Channel supplies app discovery and local artwork.':'Browser preview — sample tiles are original placeholder artwork. Real installed apps appear on the TV.');
  }
  function oledSettings() {
    const p=panel('OLED dimming','oled');
    control(p,'Dim after 30 minutes idle','moon',prefs.oledDimming?'On':'Off',()=>{prefs.oledDimming=!prefs.oledDimming;save();oledSettings();});
    control(p,'Preview dimming','moon','Wake with the remote',()=>setOledDimmed(true));
    note(p,'A near-black overlay dims the whole launcher, including menus and the screen saver. Press a remote button or move the pointer to wake it. The first press only wakes the screen.');
    note(p,'Applies while Aerial Home is on screen. Keep LG’s built-in OLED care enabled; dimming reduces light output but cannot guarantee protection against burn-in.');
  }
  function aerialSettings() {
    const p=panel('Aerials','aerials');
    control(p,'Video background','picture',prefs.aerial?'On':'Off',()=>{prefs.aerial=!prefs.aerial;save();if(prefs.aerial)playScene();else stopVideo();aerialSettings();});
    control(p,'Shuffle','controls',prefs.shuffle?'On · no repeats':'Off · in order',()=>{prefs.shuffle=!prefs.shuffle;save();aerialSettings();});
    control(p,'Next aerial','picture',currentScene?.title || '',()=>playScene(null,true));
    control(p,'Quality','tv',{'auto':'Maximum compatible','dolby':'Prefer Dolby Vision','hdr':'Prefer HDR10 / HLG','sdr':'4K SDR'}[prefs.quality],qualitySettings);
    control(p,'Browse aerials','picture',scenes.length+' catalogue entries',()=>browseAerials());
    control(p,'Shuffle categories','picture',prefs.categories.length+' selected',aerialCategories);
    control(p,'Include legacy HD','tv',prefs.legacyHD?'On':'Off · 4K only in shuffle',()=>{prefs.legacyHD=!prefs.legacyHD;save();aerialSettings();});
    control(p,'Compatibility helper','tv',prefs.mediaServer?'Connected on your network':'Optional · newer Mac aerials',mediaServerSettings);
    section(p,'SCREEN SAVER');
    control(p,'Start after','moon',prefs.saverMinutes?prefs.saverMinutes+' minutes':'Never',()=>{const choices=[0,1,3,5,10];prefs.saverMinutes=choices[(choices.indexOf(prefs.saverMinutes)+1)%choices.length];save();aerialSettings();});
    control(p,'Start screen saver','moon','',()=>{closePanel();startSaver();});
    note(p,(preparingTitle?'Preparing '+preparingTitle+' on the Pi. ':'')+'Playing '+(currentVariant?qualityLabel(currentVariant):'no video')+'. '+(currentScene?.title || ''));
    note(p,'Apple TV and Mac catalogues are included. The highest compatible source is selected for each scene; older clips may only exist in HD. Removed Apple files are skipped.');
    if(!TV.live)note(p,'Most desktop browsers cannot decode these HEVC MOV streams. The TV uses its hardware video decoder.');
  }
  function qualitySettings() {
    const p=panel('Aerial quality','quality');
    [['auto','Maximum compatible'],['dolby','Prefer Dolby Vision'],['hdr','Prefer HDR10 / HLG'],['sdr','4K SDR']].forEach(([id,title])=>control(p,title,'tv',prefs.quality===id?'Selected':'',()=>{prefs.quality=id;failedVariants.clear();failedScenes.clear();save();playScene(currentScene?.id);qualitySettings();}));
    note(p,'Your TV reports: '+(caps.dolbyVision?'Dolby Vision, ':'')+(caps.hdr10?'HDR10, ':'')+'4K SDR. Dolby Vision is used only where its metadata was verified. This Apple snapshot supplies Dolby Vision HDR variants, not separate HDR10 variants; those scenes use SDR in HDR10 mode.');
  }
  function aerialCategories() {
    const p=panel('Shuffle categories','categories');
    ['Cities','Landscapes','Space','Underwater'].forEach(name=>control(p,name,'picture',prefs.categories.includes(name)?'Included':'Excluded',()=>{if(prefs.categories.includes(name)){if(prefs.categories.length===1){toast('Keep at least one category selected');return;}prefs.categories=prefs.categories.filter(c=>c!==name);}else prefs.categories.push(name);save();aerialCategories();}));
  }
  function browseAerials(category=null,page=0) {
    const p=panel(category || 'Apple aerial catalogue','browse');
    if(!category){['Cities','Landscapes','Space','Underwater'].forEach(name=>control(p,name,'picture',String(scenes.filter(s=>s.category===name).length),()=>browseAerials(name)));return;}
    const list=scenes.filter(s=>s.category===category),size=12;
    note(p,'Page '+(page+1)+' of '+Math.ceil(list.length/size));
    if(page)control(p,'Previous page','controls','',()=>browseAerials(category,page-1));
    list.slice(page*size,(page+1)*size).forEach(s=>{
      const v=mediaVariants(s,prefs.quality,caps,prefs.mediaServer)[0];
      control(p,s.title,'picture',v?qualityLabel(v):s.variants.some(v=>v.fps>60)?'Needs compatibility helper':'Unavailable from Apple',()=>{prefs.sceneId=s.id;prefs.aerial=true;save();playScene(s.id);});
    });
    if((page+1)*size<list.length)control(p,'Next page','controls','',()=>browseAerials(category,page+1));
  }
  function mediaServerSettings() {
    const p=panel('Compatibility helper','media-server'),input=el('input');input.type='text';input.value=prefs.mediaServer;input.placeholder='http://your-pi-address:8794';input.setAttribute('aria-label','Local media helper address');p.append(input);
    control(p,'Save helper address','check','',async()=>{const raw=input.value.trim().replace(/\/$/,'');const validated=clean({...prefs,mediaServer:raw}).mediaServer;if(raw&&!validated){toast('Enter an HTTP address on your local network, including its port');return;}if(validated){const r=await fetch(validated+'/api/status');if(!r.ok)throw new Error('Helper did not respond');}prefs.mediaServer=validated;failedScenes.clear();save();toast(validated?'Compatibility helper connected':'Compatibility helper disabled');});
    note(p,'Newer Mac aerials use 240 fps timing. The optional Pi helper changes timing to 29.97 fps without re-encoding or changing 4K pixels, bit depth or colour. Native TV/Dolby Vision streams go directly to Apple. New clips may need preparation before their first play.');
  }
  function homeSelection() {
    const p=panel('Apps on Home','home-selection');
    [['most-used','Most used'],['favorites','My favourites'],['all','All apps']].forEach(([id,title])=>control(p,title,'apps',prefs.homeMode===id?'Selected':'',()=>{prefs.homeMode=id;save();homeSelection();}));
    control(p,'Choose favourites','apps',String(prefs.favorites.length),favoritePicker);
    note(p,'Most used starts with Luma-style favourites and learns from successful app launches in Aerial Home. Inputs remain in Control Center; every app is available through All apps.');
  }
  function favoritePicker() {
    const p=panel('Choose favourites','favorites');
    control(p,'Done','check','',()=>{save();closePanel();});
    apps.filter(a=>a.id!=='settings'&&!a.input).forEach(a=>{const b=control(p,a.title,a.glyph,prefs.favorites.includes(a.id)?'✓':'',()=>{prefs.homeMode='favorites';prefs.favorites=prefs.favorites.includes(a.id)?prefs.favorites.filter(id=>id!==a.id):[...prefs.favorites,a.id];b.querySelector('small').textContent=prefs.favorites.includes(a.id)?'✓':'';save();});});
  }
  function allApps() {
    const p=panel('All apps','all-apps',true),grid=el('div');grid.id='all-app-grid';
    const list=visible(apps,prefs);grid.append(...list.map(tile));p.append(grid);hydrateIcons(list);
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
  function stopVideo() {playGeneration++;playRequest=false;preparingTitle='';clearTimeout(startTimer);video.loop=false;video.pause();video.removeAttribute('src');video.load();}
  function choices(scene) {return mediaVariants(scene,prefs.quality,caps,prefs.mediaServer).filter(v=>!failedVariants.has(v.url));}
  function pool() {return scenes.filter(s=>prefs.categories.includes(s.category)&&!failedScenes.has(s.id)&&choices(s).some(v=>prefs.legacyHD||v.height>=2160));}
  async function helperJSON(path) {
    const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),6500);
    try {const response=await fetch(prefs.mediaServer+path,{signal:controller.signal});if(!response.ok)throw new Error('Local compatibility helper is unavailable');return await response.json();}
    finally{clearTimeout(timer);}
  }
  async function preparedSource(scene,variant,generation) {
    if(variant.fps<=60)return variant.url;
    if(!prefs.mediaServer)throw new Error('This aerial needs the local compatibility helper');
    toast('Preparing '+scene.title+' in lossless 4K. The current aerial will keep playing.');
    preparingTitle=scene.title;video.loop=true;if(video.ended)video.play().catch(()=>{});
    const deadline=Date.now()+480000;
    while(Date.now()<deadline&&generation===playGeneration&&!document.hidden){
      const result=await helperJSON('/api/prepare/'+encodeURIComponent(scene.id));
      if(result.state==='ready')return prefs.mediaServer+result.url;
      if(result.state==='error')throw new Error('Could not prepare '+scene.title);
      await new Promise(resolve=>setTimeout(resolve,1800));
    }
    throw new Error(generation!==playGeneration?'Cancelled':'Aerial preparation timed out');
  }
  async function playScene(id=null,next=false) {
    if(!prefs.aerial||document.hidden){stopVideo();return;}
    const eligible=pool();let scene=id?sceneMap.get(id):currentScene;
    if(!scene||next){
      // Start with an immediately playable native file, then include prepared Mac files.
      scene=sceneMap.get(bag.next(eligible.map(s=>s.id),prefs.shuffle));
      if(!currentScene&&scene&&choices(scene)[0]?.fps>60){
        const index=bag.bag.findIndex(id=>choices(sceneMap.get(id))[0]?.fps<=60);
        if(index>=0){const native=bag.bag.splice(index,1)[0];bag.bag.unshift(scene.id);scene=sceneMap.get(native);bag.last=native;}
      }
    }
    if(!scene){stopVideo();toast('No compatible aerials are currently available in these categories');return;}
    const variant=choices(scene)[0];
    if(!variant){toast('This Apple aerial has no available compatible source');return;}
    if(!TV.live){currentScene=scene;currentVariant=variant;$('scene-title').textContent=scene.title;$('saver-place').textContent=scene.title;$('scene-quality').textContent=qualityLabel(variant);return;}
    const generation=++playGeneration;playRequest=true;
    try {
      const source=await preparedSource(scene,variant,generation);
      if(generation!==playGeneration||document.hidden)return;
      currentScene=scene;currentVariant=variant;bag.last=scene.id;
      bag.bag=bag.bag.filter(other=>other!==scene.id);
      $('scene-title').textContent=scene.title;$('saver-place').textContent=scene.title;$('scene-quality').textContent=qualityLabel(variant);
      preparingTitle='';clearTimeout(startTimer);video.loop=false;video.src=source;video.muted=true;startupMark('videoRequestedMs');
      startTimer=setTimeout(()=>failedVideo(),25000);
      video.play().catch(()=>{if(generation===playGeneration)failedVideo();});
    }catch(error){
      if(generation!==playGeneration)return;
      failedVariants.add(variant.url);failedScenes.add(scene.id);toast(error.message);
      if(!video.getAttribute('src')&&++videoFailed<8)playScene(null,true);
    }finally{if(generation===playGeneration){playRequest=false;preparingTitle='';video.loop=false;}}
  }
  function failedVideo() {
    if(!video.getAttribute('src'))return;
    clearTimeout(startTimer);if(currentVariant)failedVariants.add(currentVariant.url);
    if(++videoFailed>=10){stopVideo();toast('Aerial playback failed. Check your network and retry in Aerials.');return;}
    if(currentScene&&choices(currentScene).length)playScene(currentScene.id);
    else{if(currentScene)failedScenes.add(currentScene.id);playScene(null,true);}
  }
  video.onended=()=>{if(!playRequest)playScene(null,true);};video.onerror=()=>{if(video.error)failedVideo();};
  video.onplaying=()=>{
    startupMark('videoPlayingMs');
    clearTimeout(startTimer);videoFailed=0;
    // Prepare at most one likely upcoming Mac clip; keep actual playback native when possible.
    if(prefs.mediaServer){const next=bag.bag.slice(0,3).map(id=>sceneMap.get(id)).find(s=>choices(s)[0]?.fps>60);if(next)helperJSON('/api/prepare/'+encodeURIComponent(next.id)).catch(()=>{});}
  };
  document.addEventListener('visibilitychange',()=>{setOledDimmed(false);wakeKey=null;suppressWakeClick=false;lastActivity=Date.now();if(document.hidden){videoSuspended=true;stopVideo();}else{if(homeDirty)render(focusId,false);if(videoSuspended){videoSuspended=false;playScene();}clock(true);}});
  addEventListener('blur',()=>{clearTimeout(holdTimer);heldKey=null;longPress=false;wakeKey=null;suppressWakeClick=false;});
  addEventListener('pagehide',stopVideo);
  function startSaver() { if(!prefs.aerial){toast('Enable video background in Aerials first');return;}saver=true;document.body.classList.add('saver');$('saver-caption').hidden=false;$('home').setAttribute('inert','');$('toolbar').setAttribute('inert','');document.activeElement?.blur(); }
  function setOledDimmed(dimmed) {
    oledDimmed=dimmed;document.body.classList.toggle('oled-dimmed',dimmed);
    if(dimmed){clearTimeout(holdTimer);heldKey=null;longPress=false;}
  }
  function activity() {
    lastActivity=Date.now();
    const wasDimmed=oledDimmed;
    if(oledDimmed)setOledDimmed(false);
    if(saver){saver=false;document.body.classList.remove('saver');$('saver-caption').hidden=true;$('home').removeAttribute('inert');$('toolbar').removeAttribute('inert');render();return true;}
    return wasDimmed;
  }
  addEventListener('pointermove',()=>activity());
  addEventListener('pointerdown',e=>{suppressWakeClick=activity();if(suppressWakeClick){e.preventDefault();e.stopImmediatePropagation();}},true);
  addEventListener('click',e=>{if(suppressWakeClick||activity()){suppressWakeClick=false;e.preventDefault();e.stopImmediatePropagation();}},true);
  setInterval(()=>{
    if(document.hidden)return;
    const idle=Date.now()-lastActivity;
    if(prefs.oledDimming&&!oledDimmed&&idle>=30*60000)setOledDimmed(true);
    if(!oledDimmed&&!panelKind&&!saver&&prefs.saverMinutes&&idle>=prefs.saverMinutes*60000)startSaver();
  },5000);
  let heldKey=null,holdTimer,longPress=false,moveOriginal=null;
  addEventListener('keyup',e=>{
    if(wakeKey===(e.code||e.key)){wakeKey=null;e.preventDefault();e.stopImmediatePropagation();return;}
    if(e.key==='Enter' && heldKey==='Enter'){clearTimeout(holdTimer);heldKey=null;if(!longPress){e.preventDefault();document.activeElement?.click();}longPress=false;}
  },true);
  addEventListener('keydown',e=>{
    if(wakeKey===(e.code||e.key)){lastActivity=Date.now();e.preventDefault();e.stopImmediatePropagation();return;}
    if(activity()){wakeKey=e.code||e.key;e.preventDefault();e.stopImmediatePropagation();return;}
    const back=e.key==='Escape'||e.keyCode===461||e.key==='Backspace'&&!/INPUT/.test(document.activeElement?.tagName);
    if(back) {e.preventDefault();if(moving){moving=null;if(moveOriginal){prefs.order=moveOriginal;save();moveOriginal=null;}render();$('hint').textContent='↓ Inputs · ↑ Control Center';}else if(panelKind)closePanel();else if(inputRowOpen)hideInputRow();else controls();return;}
    if(e.key==='Enter' && !/INPUT/.test(document.activeElement?.tagName)) {
      e.preventDefault();if(heldKey || e.repeat)return;heldKey='Enter';longPress=false;
      const id=document.activeElement?.dataset?.id;
      if(id&&!document.activeElement.classList.contains('input-button')&&(!panelKind||panelKind==='all-apps')&&!moving)holdTimer=setTimeout(()=>{longPress=true;appMenu(apps.find(a=>a.id===id)||prefs.folders.find(f=>f.id===id));},850);
      else{longPress=true;document.activeElement?.click();}
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
      const container=document.activeElement?.parentElement;
      if(document.activeElement?.classList.contains('app')&&['all-app-grid','folder-grid'].includes(container?.id)){
        const tiles=Array.from(container.children),at=tiles.indexOf(document.activeElement),next=nextIndex(at,dir,tiles.length,4);
        if(next<0)$('panel').querySelector('.close')?.focus();else tiles[next]?.focus();return;
      }
      const i=all.indexOf(document.activeElement),delta=dir==='down'||dir==='right'?1:-1;
      all[Math.max(0,Math.min(all.length-1,i+delta))]?.focus();return;
    }
    if(document.activeElement?.classList.contains('input-button')){
      if(dir==='up'){hideInputRow();return;}
      const buttons=Array.from($('input-row').children),at=buttons.indexOf(document.activeElement);
      if(dir==='left'||dir==='right')buttons[Math.max(0,Math.min(buttons.length-1,at+(dir==='left'?-1:1)))]?.focus();return;
    }
    const current=Array.from($('grid').children),index=current.indexOf(document.activeElement);
    if(inputRowOpen&&dir==='up'){hideInputRow();return;}
    if(index<0){const buttons=[$('apps-button'),$('search-button'),$('controls-button')],at=buttons.indexOf(document.activeElement);if(dir==='down'){if(current.length)current[0].focus();else showInputRow();}else if(dir==='left'||dir==='right')buttons[Math.max(0,Math.min(2,(at<0?1:at)+(dir==='left'?-1:1)))]?.focus();return;}
    const next=nextIndex(index,dir,current.length,prefs.columns);
    if(dir==='down'&&!moving&&index>=Math.floor((current.length-1)/prefs.columns)*prefs.columns){showInputRow(index%prefs.columns);return;}
    if(next<0){if(!moving)controls();return;}
    if(moving&&next!==index) {
      if(!moveOriginal)moveOriginal=prefs.order.slice();
      const order=shown.map(a=>a.id),[id]=order.splice(index,1);order.splice(next,0,id);prefs.order=order;
      render(id);save();return;
    }
    current[next]?.focus();current[next]?.scrollIntoView({block:'nearest',behavior:prefs.reduceMotion?'auto':'smooth'});
  },true);
  const iconQueue=[],iconPending=new Set();let iconWorkers=0;
  function hydrateIcons(list) {
    for(const app of list){if(app.icon&&!app.image&&!iconPending.has(app.id)){iconPending.add(app.id);iconQueue.push(app);}}
    while(iconWorkers<2&&iconQueue.length){iconWorkers++;(async()=>{try{
      while(iconQueue.length){const a=iconQueue.shift(),source=AerialCache.iconKey(a);a.artworkKey=source;
        try{const saved=TV.live?await cache.icon(a):null;if(saved){a.image=saved.image;if(/^#[0-9a-f]{6}$/i.test(saved.color||''))a.color=saved.color;startupMark('cachedIconMs');}else{const asset={...a};a.image=await TV.icon(asset);a.color=asset.color;if(TV.live&&a.image)cache.putIcon(a,source);}}catch(_){}
        if(!a.image){iconPending.delete(a.id);continue;}
        const current=apps.find(x=>x.id===a.id);
        if(!current||(current!==a&&(current.artworkKey||AerialCache.iconKey(current))!==source)){iconPending.delete(a.id);if(current)hydrateIcons([current]);continue;}
        if(current!==a){current.image=a.image;current.color=a.color;current.artworkKey=source;}
        for(const button of document.querySelectorAll('.app')){if(button.dataset.id!==a.id)continue;const face=button.querySelector('.tile'),img=el('img');img.onload=()=>startupPaint('firstIconMs');img.src=a.image;img.alt='';img.onerror=()=>{img.remove();face.innerHTML=svg(a.glyph);};face.style.setProperty('--tile',a.color);face.replaceChildren(img);}
        iconPending.delete(a.id);
      }
    }finally{iconWorkers--;}})();}
  }
  async function refreshInventory() {
    try {
      const inventory=await TV.listApps({includeInputs:false});startupMark('inventoryMs');
      const previous=new Map(apps.map(a=>[a.id,a]));
      apps=inventory.map(a=>{const old=previous.get(a.id);if(old?.image&&(old.artworkKey||AerialCache.iconKey(old))===AerialCache.iconKey(a)){a.image=old.image;a.color=old.color;a.artworkKey=old.artworkKey;}return a;});
      if(!prefs.favorites.length){prefs.favorites=starterFavorites(apps);save();}
      persistStartup();
      const next=homeApps(apps,prefs).filter(a=>!a.input);
      if(next.length!==shown.length||next.some((a,i)=>a.id!==shown[i].id||a.title!==shown[i].title))render(focusId,!cached?.apps.length&&!panelKind);
      else{shown=next;hydrateIcons(shown);}
    }catch(error){
      if(!apps.length){apps=[{id:'settings',title:'Settings',glyph:'settings',color:'#aab2bd'}];render();}
      toast('App discovery failed: '+error.message);
    }
    // Input discovery is independent of both the first usable Home and video startup.
    inputsLoading=true;
    TV.listInputs().then(list=>{
      for(const a of list){const existing=apps.find(x=>x.id===a.id);if(existing)existing.title=a.title;else apps.push(a);}
      inputsLoading=false;
      persistStartup();if(prefs.homeMode==='all'&&!panelKind)render(focusId,false);
      if(panelKind==='inputs')inputs();
      if(inputRowOpen)renderInputRow(true);
    }).catch(()=>{inputsLoading=false;if(panelKind==='inputs')inputs();});
  }
  function routeLaunch(event) {
    try {
      if(event)activity();
      const params=event?.detail || JSON.parse(window.PalmSystem?.launchParams || '{}');
      if(event&&!params.screen){if(panelKind)closePanel();hideInputRow();}
      if(params.mediaServer!==undefined){prefs.mediaServer=clean({...prefs,mediaServer:params.mediaServer}).mediaServer;failedScenes.clear();save();}
      const routes={controls,settings,inputs,search,aerials:aerialSettings,apps:allApps};
      if(params.sceneId&&sceneMap.has(params.sceneId)){if(mediaReady)playScene(params.sceneId);else pendingSceneId=params.sceneId;}
      if(Object.hasOwn(routes,params.screen))routes[params.screen]();
    } catch (_) { /* Missing launch params simply open Home. */ }
  }
  document.addEventListener('webOSRelaunch',routeLaunch,true);
  if(cached?.apps.length){apps=cached.apps;render();}
  routeLaunch();
  function startBackgroundTasks(){
    refreshInventory();
    // Capabilities can gate format selection, but never gate Home rendering.
    TV.capabilities().then(device=>{caps=device;startupMark('capabilitiesMs');persistStartup();}).catch(()=>{}).finally(()=>{
      mediaReady=true;playScene(pendingSceneId || (prefs.shuffle?null:prefs.sceneId || null));
    });
    if(!prefs.selectionImported)TV.lumaFavorites().then(favorites=>{
      if(favorites.length)prefs.favorites=favorites;
      prefs.selectionImported=true;save();if(apps.length&&!panelKind)render(focusId,false);
    }).catch(()=>{});
  }
  // Give cached Home its first frame before starting service calls.
  if(cached?.apps.length)requestAnimationFrame(()=>setTimeout(startBackgroundTasks,0));else startBackgroundTasks();
})();
