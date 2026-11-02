'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),zlib=require('node:zlib'),{spawnSync}=require('node:child_process');
test('native package is isolated from production and preserves the executable mode',t=>{
  const temp=fs.mkdtempSync(path.join(os.tmpdir(),'aerial-native-package-'));
  t.after(()=>{if(!path.resolve(temp).startsWith(path.resolve(os.tmpdir())+path.sep))throw Error('Unexpected temporary path');fs.rmSync(temp,{recursive:true,force:true});});
  const elf=Buffer.alloc(64);elf.write('7f454c46',0,'hex');elf[4]=1;elf[5]=1;elf.writeUInt16LE(40,18);
  const input=path.join(temp,'fixture');fs.writeFileSync(input,elf);
  const tool=path.join(__dirname,'../tools/package-native.cjs'),result=spawnSync(process.execPath,[tool,input,temp],{encoding:'utf8',windowsHide:true});assert.equal(result.status,0,result.stderr);
  const ar=fs.readFileSync(path.join(temp,'io.github.justmaaarco.aerialhome.nativeprobe_0.1.0_arm.ipk'));
  let data;for(let at=8;at<ar.length;){const name=ar.toString('ascii',at,at+16).trim(),size=Number(ar.toString('ascii',at+48,at+58));if(name==='data.tar.gz/')data=zlib.gunzipSync(ar.subarray(at+60,at+60+size));at+=60+size+size%2;}
  assert.ok(data);let executable=false,metadata=false;
  for(let at=0;at+512<=data.length;){const name=data.toString('utf8',at,at+100).split('\0')[0];if(!name)break;const size=parseInt(data.toString('ascii',at+124,at+136),8);assert.ok(name.startsWith('./usr/palm/applications/io.github.justmaaarco.aerialhome.nativeprobe/'));
    if(name.endsWith('/aerial-native-probe')){assert.equal(parseInt(data.toString('ascii',at+100,at+108),8),0o755);executable=true;}
    if(name.endsWith('/appinfo.json')){const info=JSON.parse(data.subarray(at+512,at+512+size));assert.equal(info.type,'native');assert.notEqual(info.id,'io.github.justmaaarco.aerialhome');metadata=true;}
    at+=512+Math.ceil(size/512)*512;
  }
  assert.ok(executable&&metadata);
  elf[4]=2;fs.writeFileSync(input,elf);const rejected=spawnSync(process.execPath,[tool,input,temp],{encoding:'utf8',windowsHide:true});assert.notEqual(rejected.status,0);assert.match(rejected.stderr,/32-bit little-endian ARM/);
});
