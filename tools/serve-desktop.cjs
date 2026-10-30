'use strict';
const http=require('node:http'),fs=require('node:fs'),path=require('node:path');
const root=path.resolve(process.argv[2]||path.join(__dirname,'../dist/aerial-desktop')),port=Number(process.env.PORT||5190);
const types={'.html':'text/html','.js':'text/javascript','.css':'text/css','.woff2':'font/woff2','.webm':'video/webm','.json':'application/json'};
http.createServer((req,res)=>{
  if(!['GET','HEAD'].includes(req.method)){res.writeHead(405).end();return;}
  let file;try{const name=decodeURIComponent(new URL(req.url,'http://localhost').pathname);file=path.resolve(root,'.'+(name==='/'?'/index.html':name));if(!file.startsWith(root+path.sep))throw Error('Path');}catch(_){res.writeHead(400).end();return;}
  fs.stat(file,(error,stat)=>{
    if(error||!stat.isFile()){res.writeHead(404).end();return;}
    const headers={'Content-Type':types[path.extname(file)]||'application/octet-stream','Cache-Control':'no-store','X-Content-Type-Options':'nosniff','Accept-Ranges':'bytes'};
    let start=0,end=stat.size-1,status=200;
    if(req.headers.range){const range=/^bytes=(\d+)-(\d*)$/.exec(req.headers.range);if(!range){res.writeHead(416,{'Content-Range':'bytes */'+stat.size}).end();return;}start=Number(range[1]);end=range[2]?Math.min(Number(range[2]),end):end;if(start>end||start>=stat.size){res.writeHead(416,{'Content-Range':'bytes */'+stat.size}).end();return;}headers['Content-Range']='bytes '+start+'-'+end+'/'+stat.size;status=206;}
    headers['Content-Length']=Math.max(0,end-start+1);res.writeHead(status,headers);if(req.method==='HEAD'||!stat.size){res.end();return;}
    const stream=fs.createReadStream(file,{start,end});stream.on('error',()=>res.destroy());res.on('close',()=>stream.destroy());stream.pipe(res);
  });
}).listen(port,'127.0.0.1',()=>console.log('Desktop preview: http://127.0.0.1:'+port+'/?preview=1'));
