'use strict';
// Additional privacy guard for tracked files; Gitleaks handles credential patterns in history.
const fs=require('node:fs'),path=require('node:path'),{execFileSync}=require('node:child_process');
const root=path.resolve(__dirname,'..');
const files=execFileSync('git',['ls-files','-z'],{cwd:root}).toString().split('\0').filter(Boolean);
const rules=[
  ['private network address',/\b(?:192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})\b/],
  ['hardware address',/\b(?:[a-fA-F0-9]{2}[:-]){5}[a-fA-F0-9]{2}\b/],
  ['private key',/-----BEGIN (?:OPENSSH |RSA |EC |DSA )?PRIVATE KEY-----/],
  ['personal Windows path',/[A-Za-z]:[\\/]Users[\\/][^\s/\\]+/],
  ['device key filename',/tv_ed[0-9]{5}/]
];
const failures=[];
for(const file of files){
  if(/(^|\/)(private|work|captures|backups)(\/|$)|(^|\/)\.env(?:\.|\/|$)|(^|\/)config\.local\.|\.(pem|key|p12|pfx|sqlite\w*|mov|mp4|m4v|mkv|ipk)$/i.test(file))failures.push({file,reason:'private/runtime artifact filename'});
  const full=path.join(root,file);if(!fs.existsSync(full))continue;
  const data=fs.readFileSync(full);if(data.subarray(0,4096).includes(0))continue;
  const text=data.toString();for(const [reason,pattern] of rules)if(pattern.test(text))failures.push({file,reason});
}
if(failures.length){for(const f of failures)console.error(f.file+': '+f.reason);process.exitCode=1;}
else console.log('Public-file privacy check passed ('+files.length+' tracked files). Review screenshots visually before publication.');
