import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pngPixels } from './png-pixels.mjs';
const session=process.env.GALLERY_CORNER_BROWSER_SESSION ?? 'gallery-rounded-corners-visible';
const executable=process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA,'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const artifacts=resolve('test-results/task-gallery/corners'); mkdirSync(artifacts,{recursive:true});
function browser(args) {
  const run=spawnSync(executable,['--session',session,'--headed',...args,'--json'],{encoding:'utf8',timeout:45000,maxBuffer:16*1024*1024});
  if(run.status!==0) throw new Error(run.error?.message || run.stderr || run.stdout);
  const reply=JSON.parse(run.stdout); if(!reply.success) throw new Error(reply.error); return reply.data;
}
const evaluate=code=>browser(['eval','-b',Buffer.from(code).toString('base64')]).result;
const results=[];
try {
  for(const theme of ['dark','light']) for(const zoom of [1,1.25,1.5,2]) for(const kind of ['gallery','single']) {
    browser(['open',kind==='gallery'?'http://127.0.0.1:4179/?surfaceOnly=1':'http://127.0.0.1:4180/']);
    browser(['set','viewport','1280','760']);
    browser(['wait','--fn',kind==='gallery'?'Boolean(window.galleryFixture && document.querySelector(".task-gallery-panel"))':'window.previewReady === true']);
    const selector=kind==='gallery'?'.task-gallery-panel':'.preview-surface';
    evaluate(`(async()=>{
      ${kind==='gallery'?`const f=window.galleryFixture;f.configure(960,600,'${theme}');await f.open(true,f.rows);`:`await window.previewFixture.scenario('base-${theme}','captured-image',false,true);`}
      document.documentElement.style.background='#e020b0'; document.body.style.background='transparent';
      document.body.style.padding='0';document.body.style.zoom='${zoom}';
      const host=document.querySelector('#surface');host.style.margin='0';host.style.position='absolute';host.style.left='32px';host.style.top='32px';
      document.activeElement?.blur();await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
    })()`);
    const geometry=evaluate(`(()=>{const p=document.querySelector('${selector}'),r=p.getBoundingClientRect(),s=getComputedStyle(p);return {left:r.left,top:r.top,width:r.width,height:r.height,radius:parseFloat(s.borderRadius),shadow:s.boxShadow,viewportWidth:innerWidth};})()`);
    const label=`${kind}-${theme}-${zoom}`, actualPath=resolve(artifacts,`${label}-actual.png`);
    browser(['screenshot',actualPath]);
    // Diagnostic reference ONLY: keep exactly the production outer drop shadow,
    // remove panel fill/keyline/inset paint. This distinguishes a forbidden hard
    // corner from intended soft shadow; acceptance uses untouched actual image.
    evaluate(`(async()=>{const p=document.querySelector('${selector}'),s=getComputedStyle(p);let depth=0,part='',parts=[];
      for(const c of s.boxShadow){if(c==='(')depth++;if(c===')')depth--;if(c===','&&depth===0){parts.push(part);part='';}else part+=c;}parts.push(part);
      p.style.boxShadow=parts.filter(v=>!v.includes('inset')).join(',');p.style.background='transparent';p.style.borderColor='transparent';
      [...p.children].forEach(c=>c.style.visibility='hidden');
      await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
      await Promise.all(p.getAnimations().map(a=>a.finished.catch(()=>{})));
      await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true;})()`);
    const referencePath=resolve(artifacts,`${label}-shadow-only.png`); browser(['screenshot',referencePath]);
    const actual=pngPixels(actualPath),reference=pngPixels(referencePath),scale=actual.width/geometry.viewportWidth;
    const left=Math.round(geometry.left*scale),top=Math.round(geometry.top*scale),width=Math.round(geometry.width*scale),height=Math.round(geometry.height*scale);
    const radius=geometry.radius*zoom*scale,samples=[];
    for(let y=0;y<Math.ceil(radius);y++) for(let x=0;x<Math.ceil(radius);x++) {
      // Entire pixel safely outside round boundary; exclude its AA fringe.
      if(Math.hypot(radius-x-1,radius-y-1)<=radius+0.75) continue;
      for(const [corner,px,py] of [['TL',left+x,top+y],['TR',left+width-1-x,top+y],['BL',left+x,top+height-1-y],['BR',left+width-1-x,top+height-1-y]]) {
        const paint=actual.pixel(px,py),shadow=reference.pixel(px,py);
        samples.push({corner,x: px,y: py,paint,shadow,delta:Math.max(...paint.map((v,i)=>Math.abs(v-shadow[i])))});
      }
    }
    const framePoint=[left+Math.round(radius+4*zoom),top+Math.ceil(3*zoom)],frame=actual.pixel(...framePoint),bare=reference.pixel(...framePoint);
    const framePresent=Math.max(...frame.map((v,i)=>Math.abs(v-bare[i])))>5;
    const bad=samples.filter(s=>s.delta>2);
    results.push({name:label,pass:geometry.radius===8&&geometry.shadow.includes('inset')&&samples.length>0&&bad.length===0&&framePresent,
      geometry,sampleCount:samples.length,maxCornerDelta:Math.max(...samples.map(s=>s.delta)),bad,framePresent,actualPath,referencePath});
  }
}catch(error){results.push({name:'visible corner runner completed',pass:false,actual:String(error)});}
finally{try{browser(['close']);}catch{}}
const report={results,failed:results.filter(r=>!r.pass),evidenceBoundary:'Untouched production panel paint vs diagnostic outer-shadow-only reference on contrasting backdrop. CSS zoom 1/1.25/1.5/2, not native DPI certification. Actual PNG pixels; no copied component CSS or native shell.'};
writeFileSync(resolve(artifacts,'corner-pixels.json'),JSON.stringify(report,null,2));
console.log(JSON.stringify({total:results.length,passed:results.filter(r=>r.pass).length,failed:report.failed,artifacts},null,2));
process.exitCode=report.failed.length?1:0;
