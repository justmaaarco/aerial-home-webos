'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),zlib=require('node:zlib');
const {execFileSync}=require('node:child_process');
test('IPK gzip headers do not vary with Windows versus Linux build hosts',()=>{
  const root=path.resolve(__dirname,'..'),info=require('../app/appinfo.json');
  execFileSync(process.execPath,[path.join(root,'tools/package.cjs')],{stdio:'pipe'});
  const archive=fs.readFileSync(path.join(root,'dist',info.id+'_'+info.version+'_all.ipk'));
  assert.equal(archive.subarray(0,8).toString(),'!<arch>\n');let offset=8,compressedMembers=0;
  while(offset<archive.length){
    const header=archive.subarray(offset,offset+60),name=header.subarray(0,16).toString().trim(),size=Number(header.subarray(48,58).toString().trim()),body=archive.subarray(offset+60,offset+60+size);
    if(name.endsWith('.gz/')){compressedMembers++;assert.equal(body[9],255);assert.equal(zlib.gunzipSync(body).length%512,0);}
    offset+=60+size+size%2;
  }
  assert.equal(compressedMembers,2);assert.equal(offset,archive.length);
});
