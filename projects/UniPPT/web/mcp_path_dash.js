(function(global){
'use strict';
const node=typeof module!=='undefined'&&module.exports;
function dashGeometry(geometry,width,height,pattern,offset=0){
 const {normalizeGeometry}=node?require('./mcp_scene.js'):global.UniPptMcpScene;
 if(!Array.isArray(pattern)||pattern.length!==2||pattern.some(n=>!Number.isFinite(n)||n<.25||n>1000)||!Number.isFinite(offset)||Math.abs(offset)>1e6)throw Error('Invalid source-pixel dash pattern/offset');
 const g=normalizeGeometry(geometry),ts=g.pathData.split(' '),sx=width/g.width,sy=height/g.height,segments=[];
 let at=0,p=[0,0],start=p,active=[];
 const line=q=>{active.push([p,q]);p=q;if(active.length>20000)throw Error('Dash path complexity exceeded');};
 const dist=(a,b)=>Math.hypot(a[0]-b[0],a[1]-b[1]),mid=(a,b)=>[(a[0]+b[0])/2,(a[1]+b[1])/2];
 const flat=(a,b,c,d,depth=0)=>{if(depth>=16||dist(a,b)+dist(b,c)+dist(c,d)-dist(a,d)<.004){line(d);return;}
  const ab=mid(a,b),bc=mid(b,c),cd=mid(c,d),abc=mid(ab,bc),bcd=mid(bc,cd),m=mid(abc,bcd);flat(a,ab,abc,m,depth+1);flat(m,bcd,cd,d,depth+1);};
 const point=()=>[Number(ts[at++])*sx,Number(ts[at++])*sy];
 while(at<ts.length){const cmd=ts[at++];if(cmd==='M'){if(active.length)segments.push(active);active=[];p=point();start=p;}else if(cmd==='L')line(point());else if(cmd==='C'){const a=p,b=point(),c=point(),d=point();flat(a,b,c,d);}else if(cmd==='Z')line(start);else throw Error('Unexpected normalized dash command');}
 if(active.length)segments.push(active);
 const total=pattern[0]+pattern[1],out=[],fmt=n=>Number(n.toFixed(4));
 for(const lines of segments){let phase=((offset%total)+total)%total,index=phase<pattern[0]?0:1,remain=(index?total:pattern[0])-phase,open=false;
  for(const [a,b]of lines){const length=dist(a,b);if(length<1e-8)continue;let pos=0;while(pos<length-1e-8){const step=Math.min(remain,length-pos),q=t=>[fmt(a[0]+(b[0]-a[0])*t/length),fmt(a[1]+(b[1]-a[1])*t/length)];
    if(index===0){if(!open)out.push('M',...q(pos));out.push('L',...q(pos+step));open=true;}else open=false;
    pos+=step;remain-=step;if(remain<1e-8){index=1-index;remain=pattern[index];if(index)open=false;}if(out.length>25000)throw Error('Dash path complexity exceeded');
   }}
 }
 if(!out.length)throw Error('Dash pattern produces no visible stroke');
 return normalizeGeometry({width,height,pathData:out.join(' ')});
}

global.UniPptMcpPathDash={dashGeometry};if(node)module.exports={dashGeometry};
})(globalThis);
