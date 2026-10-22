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
      saverMinutes: [0,1,3,5,10].includes(x.saverMinutes) ? x.saverMinutes : 3,
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
  const api={defaults,clean,visible,nextIndex};
  if(typeof module!=='undefined') module.exports=api;
  root.AerialCore=api;
})(typeof window!=='undefined'?window:globalThis);
