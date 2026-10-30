'use strict';
const fs=require('node:fs'),path=require('node:path');
const root=path.resolve(__dirname,'..'),destination=path.resolve(process.argv[2]&&!process.argv[2].startsWith('--')?process.argv[2]:path.join(root,'dist/aerial-desktop')),fullCatalogue=process.argv.includes('--catalogue');
const screensaver=process.argv.includes('--screensaver');
if(fs.existsSync(destination)&&fs.readdirSync(destination).length){let existing;try{existing=JSON.parse(fs.readFileSync(path.join(destination,'project.json'),'utf8'));}catch(_){}if(!['Aerial Desktop','Aerial Desktop Screensaver'].includes(existing?.title))throw Error('Destination must be empty or an Aerial Desktop package');}
fs.mkdirSync(destination,{recursive:true});
for(const name of ['index.html','desktop.css','desktop.js'])fs.copyFileSync(path.join(root,'desktop',name),path.join(destination,name));
for(const name of ['core.js','media.js'])fs.copyFileSync(path.join(root,'app',name),path.join(destination,name));
for(const name of ['InterVariable.woff2','Inter-LICENSE.txt'])fs.copyFileSync(path.join(root,'app/assets',name),path.join(destination,name));
fs.copyFileSync(path.join(root,'LICENSE'),path.join(destination,'LICENSE'));
const combo=(text,value,options,order)=>({type:'combo',text,value,options:options.map(([value,label])=>({value,label})),order});
const bool=(text,value,order)=>({type:'bool',text,value,order});
const properties={
  ...(screensaver?{
  clockstyle:combo('Clock style','glass',[['classic','Classic'],['minimal','Minimal'],['glass','Liquid Glass'],['immersive','Immersive']],0),
  clockformat:combo('Time format','auto',[['auto','System'],['12','12-hour'],['24','24-hour']],1),
  clockposition:combo('Clock position','top',[['top','Upper center'],['center','Center'],['right','Upper right']],2),
  showdate:bool('Show date',true,3)}:{}),shuffle:bool('Shuffle aerials',true,4),oledcare:bool(screensaver?'OLED clock care':'OLED text care',true,5)
};
fs.writeFileSync(path.join(destination,'project.json'),JSON.stringify({title:screensaver?'Aerial Desktop Screensaver':'Aerial Desktop',description:(fullCatalogue?'Catalogue aerials with on-demand playback':'Local aerials')+(screensaver?' and selectable clocks.':' without a desktop clock.')+' Independent open-source desktop companion to Aerial Home.',type:'web',file:'index.html',visibility:'private',version:1,general:{properties,supportsaudioprocessing:false}},null,2)+'\n');
fs.writeFileSync(path.join(destination,'mode.js'),'window.AERIAL_DESKTOP_SCREENSAVER='+screensaver+';\n');
if(!fs.existsSync(path.join(destination,'playlist.js')))fs.writeFileSync(path.join(destination,'playlist.js'),'window.AERIAL_DESKTOP_PLAYLIST=[];\n');
if(fullCatalogue){
  const vm=require('node:vm'),scope={window:{}};vm.runInNewContext(fs.readFileSync(path.join(root,'app/catalogue.js'),'utf8'),scope);
  const list=scope.window.APPLE_AERIALS.scenes.map(s=>({id:s.id,title:s.title,category:s.category,streaming:true,local:fs.existsSync(path.join(destination,'videos',s.id+'.webm')),file:'http://127.0.0.1:5194/video/'+s.id+'.webm'}));
  fs.writeFileSync(path.join(destination,'playlist.js'),'window.AERIAL_DESKTOP_CATALOGUE_MODE=true;window.AERIAL_DESKTOP_PLAYLIST='+JSON.stringify(list)+';\n');
}
fs.mkdirSync(path.join(destination,'videos'),{recursive:true});console.log('Built '+destination);
