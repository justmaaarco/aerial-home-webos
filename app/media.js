(function(root){
  'use strict';
  class ShuffleBag {
    constructor(random=Math.random){this.random=random;this.bag=[];this.signature='';this.last=null;}
    next(ids,shuffle=true) {
      if(!ids.length)return null;
      const signature=ids.join('|')+shuffle;
      if(signature!==this.signature){this.bag=[];this.signature=signature;}
      if(!shuffle){this.last=ids[(ids.indexOf(this.last)+1)%ids.length];return this.last;}
      if(!this.bag.length){
        this.bag=ids.slice();
        for(let i=this.bag.length-1;i>0;i--){const j=Math.floor(this.random()*(i+1));[this.bag[i],this.bag[j]]=[this.bag[j],this.bag[i]];}
        if(this.bag.length>1&&this.bag[0]===this.last)[this.bag[0],this.bag[1]]=[this.bag[1],this.bag[0]];
      }
      this.last=this.bag.shift();return this.last;
    }
  }
  function variants(scene,quality,caps,server='') {
    return scene.variants.filter(v=>v.available!==false).map(v=>({...v,dynamicRange:v.dynamicRange || (v.range==='HDR'?'HDR (unverified)':'SDR')})).filter(v=>{
      if(v.fps>60&&!server)return false;
      if(v.dynamicRange==='Dolby Vision')return quality!=='sdr' && quality!=='hdr' && caps.dolbyVision;
      if(['HDR10','HLG'].includes(v.dynamicRange))return quality!=='sdr'&&caps.hdr10;
      if(v.range==='HDR')return false; // Never present unverified Dolby profile 5 as generic HDR10.
      return true;
    }).sort((a,b)=>{
      const score=v=>v.height*100+(v.dynamicRange==='Dolby Vision'?30:['HDR10','HLG'].includes(v.dynamicRange)?20:0)+(v.fps<=60?5:0);
      return score(b)-score(a);
    });
  }
  function label(v) {return (v.height>=2160?'4K':v.height+'p')+' · '+(v.dynamicRange || v.range || 'SDR');}
  const api={ShuffleBag,variants,label};if(typeof module!=='undefined')module.exports=api;root.AerialMedia=api;
})(typeof window!=='undefined'?window:globalThis);
