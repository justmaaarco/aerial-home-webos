(function (root) {
  'use strict';
  const defaults = { appearance: 'dark', columns: 6, aerial: true, scene: 2, saverMinutes: 3, reduceMotion: false, profile: 'Home', order: [], hidden: [], folders: [] };
  function clean(raw) {
    const x = raw && typeof raw === 'object' ? raw : {};
    return {
      appearance: ['dark','light','auto'].includes(x.appearance) ? x.appearance : defaults.appearance,
      columns: x.columns === 5 ? 5 : 6,
      aerial: typeof x.aerial === 'boolean' ? x.aerial : true,
      scene: Number.isInteger(x.scene) && x.scene >= 0 && x.scene < 3 ? x.scene : 2,
      sceneId: typeof x.sceneId==='string' ? x.sceneId.slice(0,100) : '',
      shuffle: typeof x.shuffle==='boolean' ? x.shuffle : true,
      legacyHD: !!x.legacyHD,
      quality: ['auto','dolby','hdr','sdr'].includes(x.quality) ? x.quality : 'auto',
      categories: Array.isArray(x.categories) ? x.categories.filter(c=>['Cities','Landscapes','Space','Underwater'].includes(c)) : ['Cities','Landscapes','Space','Underwater'],
      mediaServer: typeof x.mediaServer==='string' && /^http:\/\/(192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|localhost|127\.0\.0\.1):\d{2,5}$/.test(x.mediaServer) ? x.mediaServer : '',
      homeMode: ['most-used','favorites','all'].includes(x.homeMode) ? x.homeMode : 'most-used',
      favorites: Array.isArray(x.favorites) ? [...new Set(x.favorites.filter(v=>typeof v==='string'))].slice(0,100) : [],
      selectionImported: !!x.selectionImported,
      usage: Object.fromEntries(Object.entries(x.usage && typeof x.usage==='object' ? x.usage : {}).filter(([k,v])=>/^[a-zA-Z0-9._-]+$/.test(k) && v && Number.isFinite(v.count) && v.count>0).slice(0,500).map(([k,v])=>[k,{count:Math.min(Math.floor(v.count),1000000),last:Number.isFinite(v.last)?v.last:0}])),
      saverMinutes: [0,1,3,5,10].includes(x.saverMinutes) ? x.saverMinutes : 3,
      oledDimming: typeof x.oledDimming==='boolean' ? x.oledDimming : true,
      reduceMotion: !!x.reduceMotion,
      profile: typeof x.profile === 'string' ? x.profile.slice(0,32) || 'Home' : 'Home',
      order: Array.isArray(x.order) ? [...new Set(x.order.filter(v=>typeof v==='string'))] : [],
      hidden: Array.isArray(x.hidden) ? [...new Set(x.hidden.filter(v=>typeof v==='string'))] : [],
      folders: Array.isArray(x.folders) ? x.folders.filter(f=>f && /^folder-[a-z0-9-]+$/.test(f.id) && typeof f.title==='string' && Array.isArray(f.apps)).map(f=>({id:f.id,title:f.title.slice(0,32),apps:[...new Set(f.apps.filter(v=>typeof v==='string' && !v.startsWith('folder-')))]})).slice(0,20) : []
    };
  }
  function visible(apps, prefs, query='', folder=null) {
    const map = new Map(apps.map(a=>[a.id,a]));
    let list;
    if (folder) list = (prefs.folders.find(f=>f.id===folder)?.apps || []).map(id=>map.get(id)).filter(Boolean);
    else {
      const grouped = new Set(prefs.folders.flatMap(f=>f.apps));
      list = apps.filter(a=>!prefs.hidden.includes(a.id) && (!grouped.has(a.id) || query));
      if (!query) list.push(...prefs.folders.filter(f=>f.apps.some(id=>map.has(id))).map(f=>({...f,folder:true})));
    }
    const rank = new Map(prefs.order.map((id,i)=>[id,i]));
    return list.filter(a=>a.title.toLowerCase().includes(query.toLowerCase())).sort((a,b)=>(rank.get(a.id)??9999)-(rank.get(b.id)??9999));
  }
  function nextIndex(index, direction, count, columns) {
    if(!count) return -1;
    const row=Math.floor(index/columns), col=index%columns;
    if(direction==='left') return col ? index-1 : index;
    if(direction==='right') return col<columns-1 && index+1<count ? index+1 : index;
    if(direction==='up') return row ? index-columns : -1;
    if(direction==='down') return Math.min(index+columns,count-1);
    return index;
  }
  function starterFavorites(apps) {
    const patterns=[/netflix/i,/youtube/i,/apple.?tv|com\.apple\.appletv/i,/disney/i,/prime.?video|^amazon$/i,/live.?tv/i];
    const ids=patterns.map(pattern=>apps.find(a=>pattern.test(a.id+' '+a.title))).filter(Boolean).map(a=>a.id);
    return ids.length?ids:apps.filter(a=>a.id.startsWith('demo-')&&!a.input).slice(0,6).map(a=>a.id);
  }
  function homeApps(apps,prefs) {
    const list=visible(apps,prefs);
    if(prefs.homeMode==='all')return list;
    const seed=prefs.favorites.length ? prefs.favorites : starterFavorites(apps);
    const rank=new Map(seed.map((id,i)=>[id,i]));
    const candidates=list.filter(a=>!a.input && a.id!=='settings' && !a.folder);
    let chosen;
    if(prefs.homeMode==='favorites')chosen=candidates.filter(a=>rank.has(a.id)).sort((a,b)=>rank.get(a.id)-rank.get(b.id));
    else chosen=candidates.filter(a=>rank.has(a.id)||prefs.usage[a.id]?.count).sort((a,b)=>(prefs.usage[b.id]?.count||0)-(prefs.usage[a.id]?.count||0)||(prefs.usage[b.id]?.last||0)-(prefs.usage[a.id]?.last||0)||(rank.get(a.id)??999)-(rank.get(b.id)??999)).slice(0,6);
    const manual=new Map(prefs.order.map((id,i)=>[id,i]));
    chosen.sort((a,b)=>(manual.get(a.id)??999)-(manual.get(b.id)??999));
    return [...chosen,...list.filter(a=>a.folder)];
  }
  function greeting(hour,rng=Math.random,previous=-1) {
    const period=hour>=5&&hour<12?'morning':hour>=12&&hour<17?'afternoon':hour>=17&&hour<22?'evening':'night';
    const phrases={
      morning:['A fresh start, at your pace.','Something good to start the day.','Ease into the day with a favourite.','A little inspiration for your morning.'],
      afternoon:['A moment to take a break.','Settle in for something you love.','Make a little time for yourself.','A good afternoon for a favourite.'],
      evening:['Your favourites, and time to unwind.','Settle in. The evening is yours.','A good story to end the day.','Find something worth staying in for.'],
      night:['One more episode?','A quiet moment, just for you.','A little company for the late hours.','Wind down with a favourite.']
    };
    let index=Math.floor(rng()*phrases[period].length)%phrases[period].length;
    if(index===previous)index=(index+1)%phrases[period].length;
    return {period,index,title:'Good '+(period==='night'?'evening':period)+'.',phrase:phrases[period][index]};
  }
  const api={defaults,clean,visible,nextIndex,homeApps,starterFavorites,greeting};
  if(typeof module!=='undefined') module.exports=api;
  root.AerialCore=api;
})(typeof window!=='undefined'?window:globalThis);
