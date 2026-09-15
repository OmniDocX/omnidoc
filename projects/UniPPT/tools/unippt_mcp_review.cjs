'use strict';
// Read-only source-aligned triage. Pixel error is a locator, never acceptance.
async function reviewRegions(sharp, sourcePath, png, regions, fit, sourceSize) {
  const started=performance.now(),native=sharp(Buffer.from(png,'base64'));
  const meta=await native.metadata();
  const scale=meta.width/fit.pageWidth;
  const aligned=await native.extract({left:Math.round(fit.left*scale),top:Math.round(fit.top*scale),width:Math.round(sourceSize.width*fit.scale*scale),height:Math.round(sourceSize.height*fit.scale*scale)}).resize(sourceSize.width,sourceSize.height).flatten({background:'#ffffff'}).toColourspace('srgb').raw().toBuffer();
  const original=await sharp(sourcePath).flatten({background:'#ffffff'}).toColourspace('srgb').raw().toBuffer();
  const scores=[];
  for(const region of regions) {
    const [x,y,w,h]=region.rect,step=Math.max(1,Math.ceil(Math.sqrt(w*h/50000)));let error=0,count=0;
    for(let yy=y;yy<y+h;yy+=step)for(let xx=x;xx<x+w;xx+=step) {
      const i=(yy*sourceSize.width+xx)*3;
      if(Math.min(original[i],original[i+1],original[i+2],aligned[i],aligned[i+1],aligned[i+2])>245)continue;
      for(let c=0;c<3;c++)error+=Math.abs(original[i+c]-aligned[i+c]);count+=3;
    }
    if(count)scores.push({...region,foregroundMeanAbsoluteRgb:error/count});
  }
  scores.sort((a,b)=>b.foregroundMeanAbsoluteRgb-a.foregroundMeanAbsoluteRgb);
  const selected=scores.slice(0,6),tiles=[];let y=0;
  for(const r of selected){const [left,top,width,height]=r.rect;
    const tileHeight=Math.min(180,Math.max(48,Math.round(height*Math.min(2,580/width))));
    for(const [i,buf]of [original,aligned].entries())tiles.push({input:await sharp(buf,{raw:{width:sourceSize.width,height:sourceSize.height,channels:3}}).extract({left,top,width,height}).resize(580,tileHeight,{fit:'contain',background:'#ffffff'}).png().toBuffer(),left:i*592,top:y});
    r.tile={top:y,height:tileHeight};y+=tileHeight+12;
  }
  const reviewPngBase64=tiles.length?(await sharp({create:{width:1172,height:y,channels:3,background:'#e5e7eb'}}).composite(tiles).png().toBuffer()).toString('base64'):undefined;
  return {reviewPngBase64,regionalReview:{layout:'Each row: source LEFT, native RIGHT; rows follow regions order. Gaps are separators, not slide content.',regions:selected,metric:'Foreground RGB absolute difference 0–255, ranking only; font anti-aliasing and intended overlaps affect score.',visualFidelityPassed:false,reviewMs:performance.now()-started}};
}
function regionRects(compiled,rows,source,page) {
  const result=[],byId=new Map(compiled.objects.map(o=>[o.id,o]));
  for(const row of rows){if(['photo','output','arrows','arrow','line','path','box'].includes(row[1]))continue;
    let x0=Infinity,y0=Infinity,x1=-Infinity,y1=-Infinity;
    for(const id of compiled.regions[row[0]]||[]){const f=byId.get(id).frame,a=(f.rotation||0)*Math.PI/180,c=Math.abs(Math.cos(a)),s=Math.abs(Math.sin(a)),w=f.width*c+f.height*s,h=f.height*c+f.width*s,cx=f.x+f.width/2,cy=f.y+f.height/2;
      x0=Math.min(x0,cx-w/2);y0=Math.min(y0,cy-h/2);x1=Math.max(x1,cx+w/2);y1=Math.max(y1,cy+h/2);}
    const {scale,left,top}=compiled.fit;
    x0=Math.max(0,Math.floor((x0-left)/scale)-3);y0=Math.max(0,Math.floor((y0-top)/scale)-3);
    x1=Math.min(source.width,Math.ceil((x1-left)/scale)+3);y1=Math.min(source.height,Math.ceil((y1-top)/scale)+3);
    if(x1>x0&&y1>y0)result.push({id:row[0],kind:row[1],rect:[x0,y0,x1-x0,y1-y0]});
  }
  return result;
}
module.exports={reviewRegions,regionRects};
