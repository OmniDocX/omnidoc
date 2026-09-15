(function(global){
  "use strict";
  // Deterministic vector recovery for flat scientific diagrams. Photographs
  // remain explicitly bounded crops; text comes from the visual inventory.
  // Never embed a slide-sized raster as a fallback.
  function simplify(points,tolerance=0){
    if(points.length<4)return points;
    const out=[points[0]];
    for(let i=1;i<points.length-1;i++){
      const a=out[out.length-1],b=points[i],c=points[i+1];
      if(Math.abs((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]))>tolerance*Math.max(1,Math.hypot(c[0]-a[0],c[1]-a[1])))out.push(b);
    }
    out.push(points[points.length-1]);return out;
  }
  function detectPhotos(data,width,height){
    const tile=8,cols=Math.ceil(width/tile),rows=Math.ceil(height/tile),texture=new Uint8Array(cols*rows);
    for(let gy=0;gy<rows;gy++)for(let gx=0;gx<cols;gx++){
      const colors=new Set();let chroma=0;
      for(let y=gy*tile;y<Math.min(height,(gy+1)*tile);y++)for(let x=gx*tile;x<Math.min(width,(gx+1)*tile);x++){
        const i=(y*width+x)*4,r=data[i],g=data[i+1],b=data[i+2];
        colors.add((r>>5)*64+(g>>5)*8+(b>>5));if(Math.max(r,g,b)-Math.min(r,g,b)>18)chroma++;
      }
      if(colors.size>=12 && chroma>=12)texture[gy*cols+gx]=1;
    }
    // Close isolated holes in the texture mask without touching flat modules.
    const joined=new Uint8Array(texture);
    for(let gy=1;gy<rows-1;gy++)for(let gx=1;gx<cols-1;gx++){
      let n=0;for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++)n+=texture[(gy+dy)*cols+gx+dx];
      if(n>=3)joined[gy*cols+gx]=1;
    }
    const seen=new Uint8Array(joined.length),candidates=[];
    for(let start=0;start<joined.length;start++){
      if(!joined[start] || seen[start])continue;
      const q=[start];seen[start]=1;let x1=cols,y1=rows,x2=0,y2=0;
      for(let i=0;i<q.length;i++){
        const p=q[i],x=p%cols,y=Math.floor(p/cols);x1=Math.min(x1,x);y1=Math.min(y1,y);x2=Math.max(x2,x+1);y2=Math.max(y2,y+1);
        for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++){const nx=x+dx,ny=y+dy,np=ny*cols+nx;if(nx>=0 && ny>=0 && nx<cols && ny<rows && joined[np] && !seen[np]){seen[np]=1;q.push(np);}}
      }
      if(q.length<12 || (x2-x1)*tile<30 || (y2-y1)*tile<20 || (x2-x1)/(y2-y1)>2.1)continue;
      // Tile bounds expanded at most one tile, then trimmed against whitespace.
      x1=Math.max(0,x1*tile-3);y1=Math.max(0,y1*tile-3);x2=Math.min(width,x2*tile+3);y2=Math.min(height,y2*tile+3);
      const nonwhite=(x,y)=>{const i=(y*width+x)*4;return Math.min(data[i],data[i+1],data[i+2])<220 && Math.max(data[i],data[i+1],data[i+2])-Math.min(data[i],data[i+1],data[i+2])>15;};
      while(x1<x2 && Array.from({length:y2-y1},(_,i)=>nonwhite(x1,y1+i)).filter(Boolean).length<3)x1++;
      while(x2>x1 && Array.from({length:y2-y1},(_,i)=>nonwhite(x2-1,y1+i)).filter(Boolean).length<3)x2--;
      while(y1<y2 && Array.from({length:x2-x1},(_,i)=>nonwhite(x1+i,y1)).filter(Boolean).length<3)y1++;
      while(y2>y1 && Array.from({length:x2-x1},(_,i)=>nonwhite(x1+i,y2-1)).filter(Boolean).length<3)y2--;
      const row=(y)=>Array.from({length:x2-x1},(_,i)=>nonwhite(x1+i,y)).filter(Boolean).length>=3;
      const col=(x)=>Array.from({length:y2-y1},(_,i)=>nonwhite(x,y1+i)).filter(Boolean).length>=3;
      for(let n=0;n<16 && y1>0 && row(y1-1);n++)y1--;
      for(let n=0;n<16 && y2<height && row(y2);n++)y2++;
      for(let n=0;n<16 && x1>0 && col(x1-1);n++)x1--;
      for(let n=0;n<16 && x2<width && col(x2);n++)x2++;
      const dark=(x,y)=>{const i=(y*width+x)*4;return Math.min(data[i],data[i+1],data[i+2])<220;};
      const grayRow=(y)=>Array.from({length:x2-x1},(_,i)=>dark(x1+i,y)).filter(Boolean).length>=Math.max(3,(x2-x1)*.18);
      const grayCol=(x)=>Array.from({length:y2-y1},(_,i)=>dark(x,y1+i)).filter(Boolean).length>=Math.max(3,(y2-y1)*.18);
      for(let n=0;n<12 && y1>0 && grayRow(y1-1);n++)y1--;
      for(let n=0;n<12 && y2<height && grayRow(y2);n++)y2++;
      for(let n=0;n<12 && x1>0 && grayCol(x1-1);n++)x1--;
      for(let n=0;n<12 && x2<width && grayCol(x2);n++)x2++;
      candidates.push({id:`photo-${candidates.length+1}`,box:[x1,y1,x2-x1,y2-y1]});
    }
    return candidates;
  }
  function trace(data,width,height,inventory,options={}){
    if(data.length!==width*height*4)throw new Error("RGBA dimensions mismatch");
    const pixels=new Uint8ClampedArray(data),excluded=new Uint8Array(width*height);
    const photos=inventory.photos || [], labels=inventory.labels || [];
    const bounds=(item,pad=0)=>{
      const f=item.box;
      let x=f[0],y=f[1],w=f[2],h=f[3];
      if(Math.abs(f[4] || 0)%180===90){x+=w/2-h/2;y+=h/2-w/2;[w,h]=[h,w];}
      return [Math.max(0,Math.floor(x-pad)),Math.max(0,Math.floor(y-pad)),Math.min(width,Math.ceil(x+w+pad)),Math.min(height,Math.ceil(y+h+pad))];
    };
    for(const p of photos){const [x1,y1,x2,y2]=bounds(p,1);for(let y=y1;y<y2;y++)for(let x=x1;x<x2;x++)excluded[y*width+x]=1;}
    // Reconstruct the local background behind recognized glyphs. Only dark
    // foreground pixels are replaced; photo boundaries are never inpainted.
    for(const label of labels){
      const [x1,y1,x2,y2]=bounds(label,2);
      let white=0,total=0;
      for(let y=y1;y<y2;y++)for(let x=x1;x<x2;x++){const i=(y*width+x)*4;total++;if(Math.min(pixels[i],pixels[i+1],pixels[i+2])>245)white++;}
      if(white/Math.max(1,total)>.58){
        for(let y=y1;y<y2;y++)for(let x=x1;x<x2;x++){if(excluded[y*width+x])continue;const i=(y*width+x)*4;pixels[i]=pixels[i+1]=pixels[i+2]=255;}
        continue;
      }
      const rgb=/^#([\da-f]{6})$/i.test(label.color || "") ? [1,3,5].map(i=>parseInt(label.color.slice(i,i+2),16)):[23,23,23];
      const colored=Math.max(...rgb)-Math.min(...rgb)>30;
      const ink=(x,y)=>{
        const i=(y*width+x)*4,r=pixels[i],g=pixels[i+1],b=pixels[i+2],max=Math.max(r,g,b),min=Math.min(r,g,b),range=max-min;
        // Dark BLUE feature bars are not black glyphs. Gray antialias pixels
        // up to near-white are glyph residue and must not become vector text.
        return (range<15 && max<251) || (max<180 && range/Math.max(1,max)<.35)
          || (colored && Math.hypot(r-rgb[0],g-rgb[1],b-rgb[2])<100);
      };
      const replacements=[];
      for(let y=y1;y<y2;y++)for(let x=x1;x<x2;x++){
        if(excluded[y*width+x] || !ink(x,y))continue;
        let nearest=null;
        for(let r=1;r<=10 && !nearest;r++){
          for(const [dx,dy] of [[0,-r],[0,r],[-r,0],[r,0],[-r,-r],[r,-r],[-r,r],[r,r]]){
            const nx=x+dx,ny=y+dy;
            if(nx<0 || ny<0 || nx>=width || ny>=height || excluded[ny*width+nx] || ink(nx,ny))continue;
            const j=(ny*width+nx)*4; nearest=[pixels[j],pixels[j+1],pixels[j+2]];break;
          }
        }
        replacements.push([(y*width+x)*4,nearest || [255,255,255]]);
      }
      for(const [i,c] of replacements){pixels[i]=c[0];pixels[i+1]=c[1];pixels[i+2]=c[2];}
    }
    // Scientific diagrams often contain one-pixel anti-aliased brackets and
    // circled glyphs. Splitting every gray shade fragments these into dust.
    // Join only sparse neutral stroke components; never flatten solid panels.
    if(options.connectThinGray){
      const seenGray=new Uint8Array(width*height),queueGray=new Int32Array(width*height);
      const gray=p=>{const i=p*4;return !excluded[p]&&Math.max(pixels[i],pixels[i+1],pixels[i+2])<215&&Math.max(pixels[i],pixels[i+1],pixels[i+2])-Math.min(pixels[i],pixels[i+1],pixels[i+2])<18;};
      for(let start=0;start<seenGray.length;start++){
        if(seenGray[start]||!gray(start))continue;let head=0,tail=1,x0=width,y0=height,x1=0,y1=0;queueGray[0]=start;seenGray[start]=1;
        while(head<tail){const p=queueGray[head++],x=p%width,y=Math.floor(p/width);x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x+1);y1=Math.max(y1,y+1);
          for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++){const xx=x+dx,yy=y+dy,n=yy*width+xx;if(xx<0||xx>=width||yy<0||yy>=height||seenGray[n]||!gray(n))continue;seenGray[n]=1;queueGray[tail++]=n;}
        }
        if(tail<5||tail/((x1-x0)*(y1-y0))>.4&&Math.min(x1-x0,y1-y0)>3)continue;
        const tones=Array.from(queueGray.subarray(0,tail),p=>pixels[p*4]).sort((a,b)=>a-b),tone=tones[Math.floor(tail*.3)];
        for(let k=0;k<tail;k++){const i=queueGray[k]*4;pixels[i]=pixels[i+1]=pixels[i+2]=tone;}
      }
    }
    if(options.connectThinColor){
      // White-background antialias shades share an ink direction (255-RGB).
      // Join only sparse strokes, never flatten a solid panel or photograph.
      const visited=new Uint8Array(width*height),q=new Int32Array(width*height);
      const chromatic=p=>{const i=p*4;return !excluded[p]&&Math.min(pixels[i],pixels[i+1],pixels[i+2])<235&&Math.max(pixels[i],pixels[i+1],pixels[i+2])-Math.min(pixels[i],pixels[i+1],pixels[i+2])>18;};
      for(let start=0;start<visited.length;start++){
        if(visited[start]||!chromatic(start))continue;
        const base=[0,1,2].map(k=>255-pixels[start*4+k]),bn=Math.hypot(...base);
        let head=0,tail=1,x0=width,y0=height,x1=0,y1=0;q[0]=start;visited[start]=1;
        while(head<tail){const p=q[head++],x=p%width,y=Math.floor(p/width);x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x+1);y1=Math.max(y1,y+1);
          for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++){const xx=x+dx,yy=y+dy,n=yy*width+xx;if(xx<0||xx>=width||yy<0||yy>=height||visited[n]||!chromatic(n))continue;
            const c=[0,1,2].map(k=>255-pixels[n*4+k]);if(c.reduce((sum,v,k)=>sum+v*base[k],0)/(Math.hypot(...c)*bn)<.998)continue;
            visited[n]=1;q[tail++]=n;
          }
        }
        if(tail<4||tail/((x1-x0)*(y1-y0))>.48&&Math.min(x1-x0,y1-y0)>3)continue;
        const samples=Array.from(q.subarray(0,tail)).sort((a,b)=>[0,1,2].reduce((sum,k)=>sum+pixels[a*4+k]-pixels[b*4+k],0));
        const core=samples[Math.floor(samples.length*.2)],color=[0,1,2].map(k=>pixels[core*4+k]);
        for(let k=0;k<tail;k++)for(let c=0;c<3;c++)pixels[q[k]*4+c]=color[c];
      }
    }
    const step=options.quantization || 12, tags=new Int32Array(width*height),seen=new Uint8Array(width*height);
    for(let p=0;p<tags.length;p++){
      const i=p*4;
      if(excluded[p] || pixels[i+3]<128 || Math.min(pixels[i],pixels[i+1],pixels[i+2])>246){tags[p]=-1;continue;}
      const q=(c)=>Math.min(255,Math.round(c/step)*step);
      tags[p]=(q(pixels[i])<<16)|(q(pixels[i+1])<<8)|q(pixels[i+2]);
    }
    const shapes=[],lines=[],queue=new Int32Array(width*height),minArea=options.minArea || 2;
    let region=0;
    for(let start=0;start<tags.length;start++){
      if(seen[start] || tags[start]<0)continue;
      const color=tags[start];let head=0,tail=1;queue[0]=start;seen[start]=1;
      let x1=width,y1=height,x2=0,y2=0,r=0,g=0,b=0;
      while(head<tail){
        const p=queue[head++],x=p%width,y=Math.floor(p/width),i=p*4;
        x1=Math.min(x1,x);y1=Math.min(y1,y);x2=Math.max(x2,x+1);y2=Math.max(y2,y+1);r+=pixels[i];g+=pixels[i+1];b+=pixels[i+2];
        for(const n of [x>0?p-1:-1,x<width-1?p+1:-1,y>0?p-width:-1,y<height-1?p+width:-1])if(n>=0 && !seen[n] && tags[n]===color){seen[n]=1;queue[tail++]=n;}
        if(options.connectDiagonals)for(const [dx,dy] of [[-1,-1],[1,-1],[-1,1],[1,1]]){const xx=x+dx,yy=y+dy,n=yy*width+xx;if(xx>=0&&xx<width&&yy>=0&&yy<height&&!seen[n]&&tags[n]===color){seen[n]=1;queue[tail++]=n;}}
      }
      if(tail<minArea)continue;
      const fill="#"+[r,g,b].map((v)=>Math.round(v/tail).toString(16).padStart(2,"0")).join("");
      const w=x2-x1,h=y2-y1;
      // Recover genuinely straight thin strokes as native connectors.
      if((h<=2 && w>=9 || w<=2 && h>=9) && tail/(w*h)>.8){lines.push([`trace-line-${region++}`,x1,y1+h/2,x2,y1+h/2,fill,h,null,false]);if(w<=2){lines[lines.length-1]=[`trace-line-${region-1}`,x1+w/2,y1,x1+w/2,y2,fill,w,null,false];}continue;}
      const edges=new Map(), key=(x,y)=>y*(width+1)+x;
      const add=(x,y,nx,ny)=>{const k=key(x,y);if(!edges.has(k))edges.set(k,[]);edges.get(k).push(key(nx,ny));};
      for(let q=0;q<tail;q++){
        const p=queue[q],x=p%width,y=Math.floor(p/width);
        if(y===0 || tags[p-width]!==color)add(x,y,x+1,y);
        if(x===width-1 || tags[p+1]!==color)add(x+1,y,x+1,y+1);
        if(y===height-1 || tags[p+width]!==color)add(x+1,y+1,x,y+1);
        if(x===0 || tags[p-1]!==color)add(x,y+1,x,y);
      }
      const loops=[];
      while(edges.size){
        const first=edges.keys().next().value;let current=first,points=[],guard=0;
        do{
          points.push([current%(width+1),Math.floor(current/(width+1))]);
          const next=edges.get(current);if(!next?.length)break;
          const n=next.pop();if(!next.length)edges.delete(current);current=n;
        }while(current!==first && guard++<width*height*4);
        if(points.length>=3)loops.push(simplify(points).map((p)=>[p[0]-x1,p[1]-y1]));
      }
      const path=loops.map((points)=>points.map((p,i)=>`${i?"L":"M"} ${p[0]} ${p[1]}`).join(" ")+" Z").join(" ");
      if(path.length>16000)throw new Error("矢量轮廓过于复杂，需要更细的照片或纹理区域分割");
      const shape=[`trace-region-${region++}`,"path",x1,y1,w,h,fill,"transparent",0,path];shape.area=tail;shapes.push(shape);
      if(shapes.length+lines.length>5000)throw new Error("纹理区域过多，不能安全转换为原生矢量");
    }
    // Join only nearby tiny antialias islands of the same coarse color. This
    // keeps boundaries native without thousands of one-pixel editing handles.
    const groups=new Map(),joined=[];
    for(const s of shapes){
      if(s.area>=100){joined.push(s);continue;}
      const rgb=[1,3,5].map(i=>parseInt(s[6].slice(i,i+2),16));
      const key=[Math.floor((s[2]+s[4]/2)/128),Math.floor((s[3]+s[5]/2)/128),...rgb.map(v=>Math.round(v/32))].join(",");
      if(!groups.has(key))groups.set(key,[]);groups.get(key).push(s);
    }
    for(const group of groups.values()){
      if(group.length===1){joined.push(group[0]);continue;}
      const x=Math.min(...group.map(s=>s[2])),y=Math.min(...group.map(s=>s[3]));
      const right=Math.max(...group.map(s=>s[2]+s[4])),bottom=Math.max(...group.map(s=>s[3]+s[5]));
      const paths=group.map(s=>{let n=0;return s[9].replace(/[-+]?(?:\d*\.)?\d+/g,v=>String(Number(v)+(n++%2?s[3]-y:s[2]-x)));}).join(" ");
      joined.push([group[0][0],"path",x,y,right-x,bottom-y,group[0][6],"transparent",0,paths]);
    }
    if(joined.length+lines.length>740)throw new Error(`矢量区域仍有 ${joined.length+lines.length} 个，需要更准确的文字/照片分割`);
    return {shapes:joined,lines,report:{regions:joined.length,lines:lines.length,rawRegions:shapes.length,method:"source-pixel-flat-color-trace",quantization:step,minArea}};
  }
  async function browserPlan(attachment,inventory){
    const image=new Image(); await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=reject;image.src=attachment.dataUrl;});
    const canvas=document.createElement("canvas");canvas.width=attachment.width;canvas.height=attachment.height;
    const ctx=canvas.getContext("2d");ctx.drawImage(image,0,0);
    return trace(ctx.getImageData(0,0,canvas.width,canvas.height).data,canvas.width,canvas.height,inventory);
  }
  async function browserPixels(attachment){
    const image=new Image();await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=reject;image.src=attachment.dataUrl;});
    const canvas=document.createElement("canvas");canvas.width=attachment.width;canvas.height=attachment.height;
    const ctx=canvas.getContext("2d");ctx.drawImage(image,0,0);
    return ctx.getImageData(0,0,canvas.width,canvas.height).data;
  }
  function paintAnchorSheet(canvas,image,anchors){
    const cols=3,cw=390,ch=74;canvas.width=cols*cw;canvas.height=Math.ceil(anchors.length/cols)*ch;
    const ctx=canvas.getContext("2d");ctx.fillStyle="#fff";ctx.fillRect(0,0,canvas.width,canvas.height);
    anchors.forEach((a,i)=>{
      const x=(i%cols)*cw,y=Math.floor(i/cols)*ch,b=a.pixelBox,sw=b[2]-b[0],sh=b[3]-b[1];
      ctx.fillStyle="#174aa0";ctx.font="15px Arial";ctx.fillText(a.id,x+4,y+18);
      const vertical=a.rotation!==0,iw=vertical?sh:sw,ih=vertical?sw:sh,scale=Math.min(2.5,330/iw,55/ih);
      ctx.save();ctx.translate(x+50+iw*scale/2,y+ch/2);ctx.rotate(vertical?Math.PI/2:0);
      ctx.drawImage(image,b[0],b[1],sw,sh,-sw*scale/2,-sh*scale/2,sw*scale,sh*scale);ctx.restore();
      ctx.strokeStyle="#dde2ea";ctx.strokeRect(x,y,cw,ch);
    });
  }
  async function browserAnchorSheet(attachment,anchors){
    const image=new Image();await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=reject;image.src=attachment.dataUrl;});
    const canvas=document.createElement("canvas");paintAnchorSheet(canvas,image,anchors);return canvas.toDataURL("image/png");
  }
  global.UniPptImageNativeTracer=Object.freeze({trace,browserPlan,browserPixels,simplify,detectPhotos,paintAnchorSheet,browserAnchorSheet});
})(globalThis);
