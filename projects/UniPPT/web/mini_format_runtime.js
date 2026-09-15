(function(global){
  'use strict';
  function placement(anchor,size,bounds){
    const pad=8,gap=10,width=Math.min(size.width,Math.max(1,bounds.right-bounds.left-pad*2));
    const x=Math.max(bounds.left+pad,Math.min(anchor.left,bounds.right-width-pad));
    const above=anchor.top-size.height-gap;
    const y=above>=bounds.top+pad?above:Math.min(anchor.bottom+gap,bounds.bottom-size.height-pad);
    return{x,y:Math.max(bounds.top+pad,y),width};
  }
  function canFormat(object){return Boolean(object&&!['image','media','math','table','chart','group','connector'].includes(object.kind)&&(object.kind==='text'||object.text||object.textParagraphs?.some(p=>p.runs?.some(r=>r.text))));}
  function rangeFor(saved,object){return saved?.objectId===object?.id&&saved.end>saved.start&&saved.start>=0&&saved.end<=String(object.text||'').length?saved:null;}
  const api={placement,canFormat,rangeFor};
  global.UniPptMiniFormat=Object.freeze(api);if(typeof module!=='undefined')module.exports=api;
})(globalThis);
