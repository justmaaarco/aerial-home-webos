(function () {
  'use strict';
  const live = typeof window.PalmServiceBridge === 'function';
  const active = new Set();
  function call(uri, payload={}, timeout=6500) {
    if (!live) return Promise.reject(new Error('This control needs an LG TV'));
    return new Promise((resolve,reject)=>{
      const bridge = new window.PalmServiceBridge(); active.add(bridge);
      let done=false;
      function finish(error,data) {
        if(done) return; done=true; clearTimeout(timer); active.delete(bridge);
        try { bridge.cancel(); } catch (_) { /* Older bridges may not implement cancel. */ }
        error ? reject(error) : resolve(data);
      }
      const timer=setTimeout(()=>finish(new Error('The TV did not respond')),timeout);
      bridge.onservicecallback=raw=>{
        try { const data=JSON.parse(raw); finish(data.returnValue===false ? new Error(data.errorText || 'TV request denied') : null,data); }
        catch (error) { finish(error); }
      };
      try { bridge.call(uri,JSON.stringify(payload)); } catch(error) { finish(error); }
    });
  }
  // Only fixed commands are accepted. Never interpolate app titles, search text or URLs into a shell.
  const commands = {
    apps: "luna-send -n 1 luna://com.webos.applicationManager/listLaunchPoints '{}'",
    inputs: "luna-send -n 1 luna://com.webos.service.eim/getAllInputStatus '{}'",
    capabilities: "luna-send -n 1 luna://com.webos.service.config/getConfigs '{\"configNames\":[\"tv.model.supportHDR\",\"tv.config.supportDolbyHDRContents\",\"tv.hw.panelResolution\"]}'",
    favorites: "if [ -f /media/developer/apps/usr/palm/applications/com.ecklf.luma/preferences.json ]; then cat /media/developer/apps/usr/palm/applications/com.ecklf.luma/preferences.json; else printf '{}'; fi"
  };
  async function rootCommand(name) {
    if(!Object.hasOwn(commands,name)) throw new Error('Unknown TV request');
    const r=await call('luna://org.webosbrew.hbchannel.service/exec',{command:commands[name]});
    if(r.errorCode || r.returnValue===false) throw new Error(r.errorText || 'Homebrew access unavailable');
    return JSON.parse(r.stdoutString);
  }
  const samples = [
    {id:'demo-tv',title:'TV',glyph:'tv',color:'#101114'},
    {id:'demo-music',title:'Music',glyph:'music',color:'#f04265'},
    {id:'demo-photos',title:'Photos',glyph:'photos',color:'#f4f4f8'},
    {id:'demo-video',title:'Cinema',glyph:'play',color:'#5368e4'},
    {id:'demo-games',title:'Games',glyph:'game',color:'#f4a22c'},
    {id:'settings',title:'Settings',glyph:'settings',color:'#aab2bd'},
    {id:'demo-stream',title:'Streaming',glyph:'play',color:'#bc1843'},
    {id:'demo-podcasts',title:'Podcasts',glyph:'audio',color:'#aa4de0'},
    {id:'demo-sport',title:'Sports',glyph:'sport',color:'#369b61'},
    {id:'demo-hdmi1',title:'HDMI 1',glyph:'input',color:'#27354c',input:true},
    {id:'demo-hdmi2',title:'HDMI 2',glyph:'input',color:'#27354c',input:true},
    {id:'demo-browser',title:'Browser',glyph:'globe',color:'#2191d4'}
  ];
  async function listApps({includeInputs=true}={}) {
    if(!live) return samples.map(a=>({...a}));
    let r;
    try { r=await call('luna://com.webos.applicationManager/listLaunchPoints'); }
    catch (_) { r=await rootCommand('apps'); }
    const apps=(r.launchPoints || []).filter(a=>a.id && a.title && a.visible!==false && a.id!=='io.github.justmaaarco.aerialhome').map(a=>({
      id:a.id,title:a.title,icon:a.largeIcon || a.icon,iconBase:a.folderPath || '',
      glyph:/hdmi/.test(a.id)?'input':'play',input:/hdmi/.test(a.id),color:'#394b68'
    }));
    if(includeInputs)for(const input of await listInputs().catch(()=>[]))if(!apps.some(a=>a.id===input.id))apps.push(input);
    apps.push({id:'settings',title:'Settings',glyph:'settings',color:'#aab2bd'});
    return apps;
  }
  async function listInputs() {
    if(!live)return samples.filter(a=>a.input).map(a=>({...a}));
    const e=await rootCommand('inputs');
    return (e.devices || e.deviceList || []).map(input=>({id:input.appId || input.app_id,title:input.label || input.name,glyph:'input',input:true,color:'#27354c'})).filter(a=>/^com\.(webos|palm)\.app\.hdmi[1-4]$/.test(a.id)).map(a=>({...a,title:a.title || a.id.split('.').pop().toUpperCase()}));
  }
  async function icon(app) {
    if(live && /^[a-zA-Z0-9._-]+$/.test(app.id || '')) {
      try {
        let r;
        try { r=await call('luna://com.webos.applicationManager/getAppInfo',{id:app.id}); }
        catch (_) {
          const result=await call('luna://org.webosbrew.hbchannel.service/exec',{command:"luna-send -n 1 luna://com.webos.applicationManager/getAppInfo '"+JSON.stringify({id:app.id})+"'"});
          r=JSON.parse(result.stdoutString);
        }
        const info=r.appInfo || {};
        app.icon=info.extraLargeIcon || info.largeIcon || app.icon;
        app.iconBase=info.folderPath || app.iconBase;
        if(/^#[0-9a-f]{6}$/i.test(info.iconColor || ''))app.color=info.iconColor;
      } catch (_) { /* Continue with the launch-point icon on older TVs. */ }
    }
    if(!app.icon) return null;
    let path=app.icon;
    if(/^https?:\/\//.test(path)) return path;
    if(path.startsWith('file://')) path=path.slice(7);
    if(!path.startsWith('/')) path=app.iconBase+'/'+path;
    // Application assets only. Reject metacharacters and traversal before root reads.
    if(!/^\/(usr\/palm\/applications|mnt\/otncabi\/usr\/palm\/applications|media\/(developer\/apps|cryptofs\/apps|system\/apps)\/usr\/palm\/applications)\/[A-Za-z0-9_./@+-]+\.(png|jpe?g|webp|svg)$/i.test(path) || path.split('/').includes('..')) return null;
    const r=await call('luna://org.webosbrew.hbchannel.service/exec',{command:"base64 '"+path+"' | tr -d '\\n'"});
    const data=(r.stdoutString || '').trim();
    if(!data || data.length>1500000 || !/^[A-Za-z0-9+/=]+$/.test(data)) return null;
    const ext=path.split('.').pop().toLowerCase();
    return 'data:image/'+(ext==='svg'?'svg+xml':ext==='jpg'?'jpeg':ext)+';base64,'+data;
  }
  const launch=id=>call('luna://com.webos.applicationManager/launch',{id});
  async function capabilities() {
    if(!live)return {dolbyVision:false,hdr10:false,uhd:true};
    const payload={configNames:['tv.model.supportHDR','tv.config.supportDolbyHDRContents','tv.hw.panelResolution']};
    let result;
    try {result=await call('luna://com.webos.service.config/getConfigs',payload);}
    catch (_){result=await rootCommand('capabilities');}
    const c=result.configs || {};
    return {dolbyVision:c['tv.config.supportDolbyHDRContents']===true,hdr10:c['tv.model.supportHDR']===true,uhd:['UD','8K'].includes(c['tv.hw.panelResolution'])};
  }
  async function lumaFavorites() {if(!live)return [];try {const p=await rootCommand('favorites');return Array.isArray(p.favorites)?p.favorites.filter(id=>typeof id==='string'):[];}catch(_){return [];}}
  window.TV={live,call,listApps,listInputs,icon,launch,capabilities,lumaFavorites};
})();
