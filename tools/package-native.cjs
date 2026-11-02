'use strict';
// Native TV package with its established app ID; no startup or uninstall hooks.
const fs=require('node:fs'),path=require('node:path'),zlib=require('node:zlib'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..'),info=JSON.parse(fs.readFileSync(path.join(root,'native/appinfo.json')));
if(info.id!=='io.github.justmaaarco.aerialhome.nativeprobe'||info.type!=='native')throw Error('Expected separate native launcher ID');
const binary=fs.readFileSync(process.argv[2]||path.join(root,'native/target/armv7-unknown-linux-gnueabi/release/aerial-native-probe'));
if(binary.subarray(0,4).toString('hex')!=='7f454c46'||binary[4]!==1||binary[5]!==1||binary.readUInt16LE(18)!==40)throw Error('Expected a 32-bit little-endian ARM ELF binary');
function tar(files){
  const out=[];
  for(const [name,data,mode=420] of files){
    if(Buffer.byteLength(name)>100)throw Error('Tar path too long');
    const h=Buffer.alloc(512);h.write(name,0,100);
    const oct=(n,offset,length)=>h.write(n.toString(8).padStart(length-1,'0')+'\0',offset,length,'ascii');
    oct(mode,100,8);oct(0,108,8);oct(0,116,8);oct(data.length,124,12);oct(0,136,12);
    h.fill(32,148,156);h.write('0',156);h.write('ustar\0',257);h.write('00',263);
    h.write(h.reduce((a,b)=>a+b,0).toString(8).padStart(6,'0')+'\0 ',148,8);
    out.push(h,data,Buffer.alloc((512-data.length%512)%512));
  }
  return Buffer.concat([...out,Buffer.alloc(1024)]);
}
function gzip(data){const b=zlib.gzipSync(data);b[9]=255;return b;}
function member(name,data){const h=(name+'/').padEnd(16)+'0'.padEnd(12)+'0'.padEnd(6)+'0'.padEnd(6)+'100644'.padEnd(8)+String(data.length).padEnd(10)+'`\n';return Buffer.concat([Buffer.from(h),data,data.length%2?Buffer.from('\n'):Buffer.alloc(0)]);}
const prefix='./usr/palm/applications/'+info.id+'/';
const vm=require('node:vm'),scope={window:{}};vm.runInNewContext(fs.readFileSync(path.join(root,'app/catalogue.js'),'utf8'),scope);
const fontDir=path.join(root,'native/fonts');
const fonts=fs.readdirSync(fontDir).filter(name=>/\.ttf$/.test(name)||name==='Inter-LICENSE.txt').sort().map(name=>[prefix+'fonts/'+name,fs.readFileSync(path.join(fontDir,name))]);
const data=gzip(tar([
  [prefix+'appinfo.json',Buffer.from(JSON.stringify(info,null,2)+'\n')],
  [prefix+info.main,binary,493],
  [prefix+'icon.png',fs.readFileSync(path.join(root,'app/icon.png'))],
  [prefix+'LICENSE',fs.readFileSync(path.join(root,'LICENSE'))],
  [prefix+'THIRD_PARTY_NOTICES.txt',fs.readFileSync(path.join(root,'native/THIRD_PARTY_NOTICES.txt'))],
  [prefix+'catalogue.json',Buffer.from(JSON.stringify(scope.window.APPLE_AERIALS.scenes))],
  ...fonts
]));
const control=Buffer.from('Package: '+info.id+'\nVersion: '+info.version+'\nSection: misc\nPriority: optional\nArchitecture: arm\nMaintainer: Aerial Home contributors\nDescription: Native Rust TV launcher with hardware aerial video\n');
const archive=Buffer.concat([Buffer.from('!<arch>\n'),member('debian-binary',Buffer.from('2.0\n')),member('control.tar.gz',gzip(tar([['./control',control]]))),member('data.tar.gz',data)]);
const dir=path.resolve(process.argv[3]||path.join(root,'dist'));fs.mkdirSync(dir,{recursive:true});
const name=info.id+'_'+info.version+'_arm.ipk';fs.writeFileSync(path.join(dir,name),archive);
fs.writeFileSync(path.join(dir,name+'.sha256'),crypto.createHash('sha256').update(archive).digest('hex')+'  '+name+'\n');
console.log('Built '+path.join(dir,name)+' ('+archive.length+' bytes)');
