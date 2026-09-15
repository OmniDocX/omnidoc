'use strict';
// Small Markdown subset. No raw HTML, scripts or unsafe URL schemes.
const escape=s=>String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;').replaceAll("'",'&#39;');
function inline(text){
 const re=/`([^`]+)`|\[([^\]]+)\]\(([^\s)]+)\)|\*\*([^*]+)\*\*/g;let output='',at=0;
 for(const m of text.matchAll(re)){
  output+=escape(text.slice(at,m.index));at=m.index+m[0].length;
  if(m[1])output+='<code>'+escape(m[1])+'</code>';
  else if(m[4])output+='<strong>'+escape(m[4])+'</strong>';
  else if(/^(https:\/\/[^/]|\/(?!\/)|#)/.test(m[3]))output+='<a href="'+escape(m[3])+'">'+escape(m[2])+'</a>';
  else output+=escape(m[0]);
 }return output+escape(text.slice(at));
}
function render(markdown){
 const lines=markdown.split(/\r?\n/),out=[],toc=[];let i=0,section=0;
 const block=line=>/^(#{1,3} |```|\| |\d+\. |- )/.test(line);
 while(i<lines.length){
  const line=lines[i++];if(!line.trim())continue;
  if(line.startsWith('```')){const code=[];while(i<lines.length&&!lines[i].startsWith('```'))code.push(lines[i++]);if(i===lines.length)throw Error('Unclosed documentation fence');i++;out.push('<pre tabindex="0"><code>'+escape(code.join('\n'))+'</code></pre>');continue;}
  const h=/^(#{1,3}) (.*)$/.exec(line);
  if(h){const id='section-'+(++section);out.push('<h'+h[1].length+' id="'+id+'">'+inline(h[2])+'</h'+h[1].length+'>');if(h[1].length===2)toc.push({id,title:h[2]});continue;}
  if(line.startsWith('| ')&&/^\|[ :|-]+\|\s*$/.test(lines[i]||'')){
   const cells=s=>s.trim().slice(1,-1).split('|').map(v=>v.trim()),heads=cells(line),rows=[];i++;
   while((lines[i]||'').startsWith('| '))rows.push(cells(lines[i++]));
   out.push('<div class="table-scroll" role="region" aria-label="'+escape(heads.join(' / '))+'" tabindex="0"><table><thead><tr>'+heads.map(c=>'<th scope="col">'+inline(c)+'</th>').join('')+'</tr></thead><tbody>'+rows.map(r=>'<tr>'+r.map(c=>'<td>'+inline(c)+'</td>').join('')+'</tr>').join('')+'</tbody></table></div>');continue;
  }
  const list=/^(\d+\. |- )(.*)$/.exec(line);
  if(list){const ordered=list[1]!=='- ',tag=ordered?'ol':'ul',items=[list[2]],pattern=ordered?/^\d+\. (.*)$/:/^- (.*)$/;while(i<lines.length&&pattern.test(lines[i]))items.push(pattern.exec(lines[i++])[1]);out.push('<'+tag+'>'+items.map(v=>'<li>'+inline(v)+'</li>').join('')+'</'+tag+'>');continue;}
  const p=[line];while(i<lines.length&&lines[i].trim()&&!block(lines[i]))p.push(lines[i++]);out.push('<p>'+inline(p.join('\n'))+'</p>');
 }return {html:out.join('\n'),toc};
}
function page({title,description,markdown,active,appendix=''}){
 const {html,toc}=render(markdown),pages=[['guide','/docs/','使用指南'],['protocol','/mcp-protocol.html','MCP 协议'],['integration','/docs/integration.html','本机 AI 与 MCP']];
 const nav=pages.map(([id,url,label])=>'<a href="'+url+'"'+(id===active?' aria-current="page"':'')+'>'+label+'</a>').join('');
 return '<!doctype html>\n<html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="'+escape(description)+'"><title>'+escape(title)+' · UniPPT 文档</title><link rel="icon" href="/unippt-logo.svg" type="image/svg+xml"><link rel="stylesheet" href="/docs/docs.css"></head><body><a class="skip" href="#content">跳到正文</a><header class="doc-header"><a class="brand" href="/docs/"><img src="/unippt-logo.svg" width="32" height="32" alt="">UniPPT <span>文档</span></a><nav aria-label="文档分类">'+nav+'</nav><a class="editor-link" href="/">打开编辑器</a></header><div class="doc-layout"><aside><p class="eyebrow">本页目录</p><nav aria-label="本页章节">'+toc.map(t=>'<a href="#'+t.id+'">'+escape(t.title)+'</a>').join('')+'</nav><div class="downloads"><p class="eyebrow">离线材料</p><a href="/docs/unippt-integration.md" download>完整接入手册 ↗</a><a href="/mcp-tools.json" download>本机工具 Schema ↗</a><a href="/mcp-runtime-schemas.json" download>原生操作与模块 Schema ↗</a></div></aside><main id="content" tabindex="-1"><div class="doc-meta">UniPPT / '+escape(title)+' <span>更新于 2026-09-16</span></div>'+html+appendix+'<footer>文档描述当前本机版接口与安全边界；运行时能力查询优先，不包含访问凭据。<a href="#content">返回正文顶部 ↑</a></footer></main></div></body></html>\n';
}
module.exports={escape,inline,render,page};
