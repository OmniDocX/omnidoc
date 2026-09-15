(function(global){
  'use strict';
  const KEY='unippt.mcp.global-enabled.v1';
  const localOrigin=origin=>['http://127.0.0.1:8141','http://localhost:8141'].includes(origin);
  function create({origin,storage,onChange=()=>{}}){
    const key=KEY,allowed=()=>localOrigin(origin);
    let preferred=true;
    try{const saved=storage?.getItem(key);if(saved!==null&&saved!==undefined)preferred=saved==='true';}catch{}
    return Object.freeze({
      get enabled(){return allowed()&&preferred;},
      setEnabled(value){preferred=allowed()&&value===true;try{storage?.setItem(key,String(preferred));}catch{}const enabled=allowed()&&preferred;onChange(enabled);return enabled;},
      storageChanged(event){if(event.key!==key&&event.key!==null)return;preferred=event.newValue===null||event.newValue==='true';onChange(allowed()&&preferred);},
    });
  }
  const api={KEY,localOrigin,create};global.UniPptMcpConnectionPolicy=api;if(typeof module!=='undefined')module.exports=api;
})(globalThis);
