(function(global){
  'use strict';
  function install(){
    const wrap=document.getElementById('settingsWrap'),menu=document.getElementById('settingsMenu'),trigger=document.getElementById('settingsButton');
    function close(restore=false){menu.hidden=true;trigger.setAttribute('aria-expanded','false');if(restore)trigger.focus();}
    trigger.onclick=()=>{const open=menu.hidden;menu.hidden=!open;trigger.setAttribute('aria-expanded',String(open));if(open)menu.querySelector('button,a')?.focus();};
    document.addEventListener('pointerdown',event=>{if(!wrap.contains(event.target))close();});
    wrap.addEventListener('keydown',event=>{if(event.key==='Escape'){close(true);event.preventDefault();}});
    wrap.addEventListener('focusout',event=>{if(!wrap.contains(event.relatedTarget))close();});
    return Object.freeze({close});
  }
  global.UniPptLocalSettings=Object.freeze({install});
})(globalThis);
