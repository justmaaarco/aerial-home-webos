(function(root){
  'use strict';
  const KEY='aerial-startup-v1',MAX_AGE=7*86400000,MAX_ICONS=24,MAX_IMAGE=400000;
  const fields=['id','title','icon','iconBase','glyph','color','input'];
  function snapshot(value,now=Date.now()) {
    if(!value||value.version!==1||!Number.isFinite(value.savedAt)||value.savedAt>now+60000||now-value.savedAt>MAX_AGE||!Array.isArray(value.apps))return null;
    const seen=new Set(),apps=[];
    for(const a of value.apps.slice(0,300)) {
      if(!a||!/^[-a-zA-Z0-9._]+$/.test(a.id)||a.id.length>180||typeof a.title!=='string'||!a.title||seen.has(a.id))continue;
      seen.add(a.id);const app={};
      for(const key of fields)if(typeof a[key]==='string')app[key]=a[key].slice(0,key==='title'?160:1200);else if(key==='input')app.input=!!a.input;
      apps.push(app);
    }
    const c=value.caps;
    return {version:1,savedAt:value.savedAt,apps,caps:c&&['dolbyVision','hdr10','uhd'].every(k=>typeof c[k]==='boolean')?{dolbyVision:c.dolbyVision,hdr10:c.hdr10,uhd:c.uhd}:null};
  }
  const iconKey=a=>(a.iconBase||'')+'|'+(a.icon||'');
  function validIcon(row,key) {
    return row&&row.source===key&&typeof row.image==='string'&&row.image.length<=MAX_IMAGE&&/^data:image\/(png|jpeg|webp|svg\+xml);base64,[A-Za-z0-9+/=]+$/.test(row.image);
  }
  function create(storage,idb) {
    let opening;
    function db() {
      if(!opening)opening=new Promise(resolve=>{
        if(!idb){resolve(null);return;}
        let request,done=false;
        const finish=value=>{if(done){if(value)value.close();return;}done=true;clearTimeout(timer);resolve(value);};
        const timer=setTimeout(()=>finish(null),400);
        try {
          request=idb.open('aerial-artwork-v1',1);
          request.onupgradeneeded=()=>{const store=request.result.createObjectStore('icons',{keyPath:'id'});store.createIndex('touched','touched');};
          request.onsuccess=()=>{request.result.onversionchange=()=>request.result.close();finish(request.result);};
          request.onerror=request.onblocked=()=>finish(null);
        }catch(_){finish(null);}
      });
      return opening;
    }
    return {
      read(){try{return snapshot(JSON.parse(storage.getItem(KEY)));}catch(_){return null;}},
      save(apps,caps){try{const value=snapshot({version:1,savedAt:Date.now(),apps,caps});if(value?.apps.length)storage.setItem(KEY,JSON.stringify(value));}catch(_){/* Cache is optional, including on a full disk. */}},
      async icon(app) {
        const database=await db();if(!database)return null;
        return new Promise(resolve=>{
          let done=false;const finish=value=>{if(done)return;done=true;clearTimeout(timer);resolve(value);};
          const timer=setTimeout(()=>finish(null),400);
          try{const tx=database.transaction('icons','readonly'),r=tx.objectStore('icons').get(app.id);r.onsuccess=()=>finish(validIcon(r.result,iconKey(app))&&Date.now()-r.result.touched<MAX_AGE?r.result:null);r.onerror=()=>finish(null);tx.onabort=()=>finish(null);}catch(_){finish(null);}
        });
      },
      async putIcon(app,source) {
        if(!validIcon({source,image:app.image},source))return;
        const database=await db();if(!database)return;
        try {
          const tx=database.transaction('icons','readwrite'),store=tx.objectStore('icons');
          store.put({id:app.id,source,image:app.image,color:app.color,touched:Date.now()});
          const count=store.count();count.onsuccess=()=>{
            let remove=count.result-MAX_ICONS;if(remove<=0)return;
            const cursor=store.index('touched').openKeyCursor();cursor.onsuccess=()=>{const c=cursor.result;if(c&&remove-->0){store.delete(c.primaryKey);c.continue();}};
          };
          tx.onerror=()=>{};
        }catch(_){/* Rendering never depends on a successful cache write. */}
      }
    };
  }
  const api={create,snapshot,iconKey,validIcon};if(typeof module!=='undefined')module.exports=api;root.AerialCache=api;
})(typeof window!=='undefined'?window:globalThis);
