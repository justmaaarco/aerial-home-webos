'use strict';
// Reproducible standard ar/tar IPK; no SDK, install scripts or dependencies.
const fs=require('node:fs'),path=require('node:path'),zlib=require('node:zlib'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..'),app=path.join(root,'app'),info=JSON.parse(fs.readFileSync(path.join(app,'appinfo.json')));
function gzip(data){const buffer=zlib.gzipSync(data);buffer[9]=255;return buffer;} // RFC 1952: platform-neutral OS byte.
function tar(files) {
  const out=[];
  for(const [name,data] of files) {
    if(Buffer.byteLength(name)>100)throw Error('Tar path too long: '+name);
    const h=Buffer.alloc(512);h.write(name,0,100,'utf8');
    function oct(n,offset,length){h.write(n.toString(8).padStart(length-1,'0')+'\0',offset,length,'ascii');}
    oct(420,100,8);oct(0,108,8);oct(0,116,8);oct(data.length,124,12);oct(0,136,12);
    h.fill(32,148,156);h.write('0',156);h.write('ustar\0',257);h.write('00',263);
    const sum=h.reduce((a,b)=>a+b,0);h.write(sum.toString(8).padStart(6,'0')+'\0 ',148,8);
    out.push(h,data,Buffer.alloc((512-data.length%512)%512));
  }
  return Buffer.concat([...out,Buffer.alloc(1024)]);
}
function walk(dir,prefix='') { return fs.readdirSync(dir).sort().flatMap(name=>{const f=path.join(dir,name),p=prefix+name;return fs.statSync(f).isDirectory()?walk(f,p+'/'):[[p,fs.readFileSync(f)]];}); }
function member(name,data){const header=(name+'/').padEnd(16)+'0'.padEnd(12)+'0'.padEnd(6)+'0'.padEnd(6)+'100644'.padEnd(8)+String(data.length).padEnd(10)+'`\n';return Buffer.concat([Buffer.from(header),data,data.length%2?Buffer.from('\n'):Buffer.alloc(0)]);}
const prefix='./usr/palm/applications/'+info.id+'/';
const data=gzip(tar(walk(app).map(([n,d])=>[prefix+n,d])));
const control=Buffer.from('Package: '+info.id+'\nVersion: '+info.version+'\nSection: misc\nPriority: optional\nArchitecture: all\nMaintainer: Aerial Home contributors\nDescription: Independent glass-inspired webOS home screen\n');
const archive=Buffer.concat([Buffer.from('!<arch>\n'),member('debian-binary',Buffer.from('2.0\n')),member('control.tar.gz',gzip(tar([['./control',control]]))),member('data.tar.gz',data)]);
fs.mkdirSync(path.join(root,'dist'),{recursive:true});
const filename=info.id+'_'+info.version+'_all.ipk';fs.writeFileSync(path.join(root,'dist',filename),archive);
fs.writeFileSync(path.join(root,'dist',filename+'.sha256'),crypto.createHash('sha256').update(archive).digest('hex')+'  '+filename+'\n');
console.log('Built dist/'+filename+' ('+archive.length+' bytes)');
