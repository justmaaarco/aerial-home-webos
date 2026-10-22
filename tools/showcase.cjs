'use strict';
// Local-only screenshot/demo server. Presentation assets are never packaged into the TV app.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path');
const root=path.resolve(__dirname,'..'),app=path.join(root,'app'),showcase=path.join(root,'docs/showcase'),shots=path.join(root,'docs/screenshots');
const types={'.html':'text/html','.js':'text/javascript','.css':'text/css','.woff2':'font/woff2','.png':'image/png','.jpg':'image/jpeg'};
const overlay='<link rel="stylesheet" href="/showcase/demo.css">';
http.createServer((req,res)=>{
  if(!['GET','HEAD'].includes(req.method)){res.writeHead(405).end();return;}
  let name;try{name=decodeURIComponent(new URL(req.url,'http://localhost').pathname);}catch(_){res.writeHead(400).end();return;}
  const base=name.startsWith('/showcase/')?showcase:name.startsWith('/screenshots/')?shots:app;
  const relative=name.startsWith('/showcase/')?name.slice(10):name.startsWith('/screenshots/')?name.slice(13):name.slice(1)||'index.html';
  const file=path.resolve(base,relative);
  if(!file.startsWith(base+path.sep)){res.writeHead(403).end();return;}
  fs.readFile(file,(error,data)=>{
    if(error){res.writeHead(404).end();return;}
    if(file===path.join(app,'index.html'))data=Buffer.from(data.toString().replace('</head>',overlay+'</head>'));
    res.writeHead(200,{'Content-Type':types[path.extname(file)]||'application/octet-stream','Cache-Control':'no-store','X-Content-Type-Options':'nosniff','Referrer-Policy':'no-referrer'});
    res.end(req.method==='HEAD'?undefined:data);
  });
}).listen(5179,'127.0.0.1',()=>console.log('Public demo preview: http://127.0.0.1:5179'));
