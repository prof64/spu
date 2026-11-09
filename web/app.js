import init, { Lab } from './pkg/sprite_lab.js';
const $ = s => document.querySelector(s);
const $$ = s => [...document.querySelectorAll(s)];
const fixtures = [
  {id:'charizard', name:'Charizard', file:'fixtures/charizard.sprite', thumbnail:'fixtures/charizard.png'},
  {id:'cryogonal', name:'Cryogonal', file:'fixtures/cryogonal.sprite', thumbnail:'fixtures/cryogonal.png'},
  {id:'glalie', name:'Glalie', file:'fixtures/glalie.sprite', thumbnail:'fixtures/glalie.png'},
];
const defaults = {asset:0, partner:1, view:0, animation:0, battle:false, world:false, showcase:0, count:1, mixed:false, phased:false, overlap:false, zoom:1, roughness:1, transmission:1, emission:1, opacity:1, normals:1, reflectivity:1, ambient:.65, lightPower:.9, light:[.32,.23,.6], exposure:1, mode:0, background:0, probe:[.5,.5], probeFront:false, probeEnabled:false, pixelTiming:true, bloom:.2, spill:1, sourceDepth:.9, sourceOffset:0, sourceTrainer:false, trainers:true, reflectionFocus:true, rain:.7, wind:.25, wetness:.8, lights:32, reflectionSteps:16, worldSize:48};
let settings=structuredClone(defaults), lab, original, metadata, items=[], selected=0, opponent=1, layout='front', count=100;
let ready=false, playing=true, seconds=0, speed=1, orbit=false, enabled=true, last=0, samples=[], lastStats=[], frameCount=0, error=null, noticeTimer;
const asset = () => metadata?.assets[selected];
const hasBack = () => asset()?.views.some(v=>v.name.toLowerCase()==='back');
const activeView = () => asset()?.views[layout==='back'||layout==='battle'?Math.max(0,asset().views.findIndex(v=>v.name.toLowerCase()==='back')):0];
const activeClip = () => activeView()?.clips[settings.animation];
const median = a => [...a].sort((a,b)=>a-b)[Math.floor(a.length/2)] || 0;
function notice(text, failed=false) {
  clearTimeout(noticeTimer); $('#message').textContent=text; $('#message').classList.toggle('error',failed); $('#message').hidden=false;
  noticeTimer=setTimeout(()=>$('#message').hidden=true,failed?9000:3000);
}
function configure() {
  settings.animationName=layout==='world'?null:activeClip()?.name||null;
  settings.asset = layout==='battle'?opponent:selected;
  settings.partner = selected;
  settings.view = layout==='back'?Math.max(0,asset()?.views.findIndex(v=>v.name.toLowerCase()==='back')):0;
  settings.battle=layout==='battle'; settings.world=layout==='world'; settings.showcase=0;
  settings.count=settings.world?count:1; settings.mixed=settings.world; settings.phased=settings.world;
  if(lab) lab.configure(JSON.stringify({...settings,reference:!enabled}));
  if(original) original.configure(JSON.stringify({...settings,reference:true}));
  $('#light').style.left=`${settings.light[0]*100}%`; $('#light').style.top=`${settings.light[1]*100}%`;
  $('#light').hidden=!enabled||settings.lightPower===0;
}
function resize() {
  if(!lab||!original)return;
  // Both buffers use the same dimensions even if CSS distributes an odd pixel.
  const a=$('#original-stage'),b=$('#effect-stage');
  const w=Math.max(1,Math.min(a.clientWidth,b.clientWidth)),h=Math.max(1,Math.min(a.clientHeight,b.clientHeight));
  for(const [canvas,engine] of [[$('#original'),original],[$('#canvas'),lab]]) {
    if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h;engine.resize(w,h);}
  }
}
function sync() {
  $('#layout').value=layout; $('#opponent-label').hidden=layout!=='battle'; $('#opponent').value=opponent;
  $('#layout option[value="back"]').disabled=!hasBack();
  $('#count-label').hidden=layout!=='world'; $('#population').value=count; $('#animation-label').hidden=layout==='world';
  $('#document-name').textContent=layout==='battle'?`${asset()?.name||''} / ${metadata?.assets[opponent]?.name||''}`:asset()?.name||'Sprite';
  $('#environment').value=settings.background; $('#zoom').value=settings.zoom; $('#channel').value=settings.mode;
  $('#lights').value=settings.lights; $('#quality').value=settings.reflectionSteps; $('#height').value=settings.light[2]; $('#height-value').textContent=settings.light[2].toFixed(2);
  $('#effects-enabled').checked=enabled; $('#orbit').checked=orbit; $('#pixel-timing').checked=settings.pixelTiming;
  $('#play').textContent=playing?'Pause':'Play'; $('#play').setAttribute('aria-label',playing?'Pause animation':'Play animation');
  $$('[data-setting]').forEach(input=>{const key=input.dataset.setting;input.value=settings[key];$(`[data-output="${key}"]`).textContent=settings[key].toFixed(2);});
  $$('.sprite-item').forEach(b=>{const on=+b.dataset.asset===selected;b.classList.toggle('selected',on);b.setAttribute('aria-pressed',String(on));});
  $('#original-caption').textContent=items[selected]?.local===false?'Source colors':'Gen 5';
  if(metadata) {
    const url=new URL(location.href); url.search='';url.searchParams.set('pokemon',items[selected]?.id||'imported');url.searchParams.set('view',layout);
    if(layout==='battle')url.searchParams.set('opponent',items[opponent]?.id||'');
    url.searchParams.set('scene',({0:'studio',4:'cave',6:'rain'})[settings.background]||'studio');
    if(layout==='world')url.searchParams.set('count',count);
    history.replaceState(null,'',url);
  }
  configure(); resize();
}
function updateClips() {
  const clips=activeView()?.clips||[]; settings.animation=Math.min(settings.animation,Math.max(0,clips.length-1));
  $('#clip').replaceChildren(...clips.map((c,i)=>new Option(c.name,String(i),false,i===settings.animation)));
}
function updateLibrary() {
  $('#library').replaceChildren(...metadata.assets.map((a,i)=>{
    const button=document.createElement('button');button.className='sprite-item';button.dataset.asset=i;button.title=a.name;
    if(items[i]?.thumbnail){const img=document.createElement('img');img.src=items[i].thumbnail;img.alt='';button.append(img);}
    const name=document.createElement('span');name.textContent=a.name;button.append(name);button.onclick=()=>selectAsset(i);return button;
  }));
  $('#opponent').replaceChildren(...metadata.assets.map((a,i)=>new Option(a.name,String(i))));
}
function selectAsset(index) {
  selected=Math.max(0,Math.min(index,metadata.assets.length-1));settings.animation=0;seconds=0;
  if(opponent===selected&&metadata.assets.length>1)opponent=(selected+1)%metadata.assets.length;
  if(layout==='back'&&!asset().views.some(v=>v.name.toLowerCase()==='back'))layout='front';
  resetMaterials(false);updateClips();sync();
}
function resetMaterials(showNotice=false) {
  for(const key of ['roughness','transmission','emission','opacity','normals','reflectivity'])settings[key]=1;
  settings.mode=0;settings.bloom=settings.background===4?.35:.2;enabled=true;
  if(showNotice)notice('Materials reset');
}
function setLayout(value) {
  layout=({single:'front',pair:'battle',studio:'front',crowd:'world'})[value]||value;
  if(!['front','back','battle','world'].includes(layout))layout='front';
  if(layout==='back'&&!hasBack())layout='front';
  settings.animation=0;seconds=0;updateClips();sync();
}
function applyScene(background) {
  settings.background=background;orbit=false;
  Object.assign(settings,background===4?{ambient:.12,lightPower:.25,bloom:.35}:background===6?{ambient:.4,lightPower:.8,bloom:.2}:{ambient:.65,lightPower:.9,bloom:.2});
  sync();
}
function frame(now) {
  const elapsed=last?(now-last)/1000:0;last=now;
  if(ready) {
    if(playing)seconds+=Math.min(elapsed,.15)*speed;
    const clip=activeClip();
    if(layout!=='world'&&clip&&!clip.looping&&seconds>=clip.duration){seconds=clip.duration;if(playing){playing=false;sync();}}
    if(orbit){settings.light[0]=.5+Math.cos(now*.0005)*.34;settings.light[1]=.45+Math.sin(now*.0005)*.3;configure();}
    try {
      original.frame(seconds);lastStats=lab.frame(seconds);
      samples.push(elapsed*1000);if(samples.length>90)samples.shift();
      const duration=layout==='world'?12:clip?.duration||1;const local=layout==='world'||clip?.looping?seconds%duration:Math.min(seconds,duration);
      $('#timeline').max=duration;$('#timeline').value=local;$('#time').textContent=`${local.toFixed(2)} s`;
      if(++frameCount%20===0){$('#fps').textContent=`${Math.round(1000/(median(samples)||16.7))} fps`;$('#status').textContent=layout==='world'?`${count.toLocaleString()} sprites · ${lastStats[2].toLocaleString()} parts`:`${asset().name} · ${layout==='battle'?'Battle':activeView()?.name||'Front'}`;}
    } catch(e) {ready=false;error=String(e);notice(error,true);}
  }
  requestAnimationFrame(frame);
}
$('#layout').onchange=e=>setLayout(e.target.value);
$('#opponent').onchange=e=>{opponent=+e.target.value;sync();};
$('#population').onchange=e=>{count=+e.target.value;samples=[];sync();};
$('#clip').onchange=e=>{settings.animation=+e.target.value;seconds=0;sync();};
$('#zoom').onchange=e=>{settings.zoom=+e.target.value;sync();};
$('#environment').onchange=e=>applyScene(+e.target.value);
$$('[data-setting]').forEach(input=>input.oninput=()=>{settings[input.dataset.setting]=+input.value;sync();});
$('#channel').onchange=e=>{settings.mode=+e.target.value;sync();};
$('#lights').onchange=e=>{settings.lights=+e.target.value;sync();};
$('#quality').onchange=e=>{settings.reflectionSteps=+e.target.value;if(!settings.reflectionSteps)settings.reflectivity=0;else if(!settings.reflectivity)settings.reflectivity=1;sync();};
$('#height').oninput=e=>{settings.light[2]=+e.target.value;sync();};
$('#pixel-timing').onchange=e=>{settings.pixelTiming=e.target.checked;configure();};
$('#effects-enabled').onchange=e=>{enabled=e.target.checked;sync();};
$('#orbit').onchange=e=>orbit=e.target.checked;
$('#reset').onclick=()=>{resetMaterials();sync();};
$('#speed').onchange=e=>speed=+e.target.value;
$('#play').onclick=()=>{playing=!playing;if(playing&&activeClip()&&!activeClip().looping&&seconds>=activeClip().duration)seconds=0;sync();};
$('#timeline').oninput=e=>{seconds=+e.target.value;playing=false;sync();};
document.addEventListener('keydown',e=>{if(e.code==='Space'&&!['INPUT','SELECT','BUTTON','SUMMARY'].includes(e.target.tagName)){e.preventDefault();$('#play').click();}});
function moveLight(e){const r=$('#effect-stage').getBoundingClientRect();settings.light[0]=Math.max(0,Math.min(1,(e.clientX-r.left)/r.width));settings.light[1]=Math.max(0,Math.min(1,(e.clientY-r.top)/r.height));orbit=false;$('#orbit').checked=false;configure();}
for(const el of [$('#light'),$('#canvas')])el.onpointerdown=e=>{el.setPointerCapture(e.pointerId);moveLight(e);el.onpointermove=moveLight;el.onpointerup=el.onpointercancel=()=>el.onpointermove=null;};
$('#light').onkeydown=e=>{const d={ArrowLeft:[-.02,0],ArrowRight:[.02,0],ArrowUp:[0,-.02],ArrowDown:[0,.02]}[e.key];if(!d)return;e.preventDefault();settings.light[0]=Math.max(0,Math.min(1,settings.light[0]+d[0]));settings.light[1]=Math.max(0,Math.min(1,settings.light[1]+d[1]));orbit=false;$('#orbit').checked=false;configure();};
async function loadFile(file) {
  if(!file)return;
  try {
    if(file.size>32*1024*1024)throw Error('File exceeds 32 MiB.');
    const bytes=new Uint8Array(await file.arrayBuffer());
    if(!lab){await createLabs([{entry:{id:'imported',local:false},bytes}]);}
    else {const index=lab.load(bytes);original.load(bytes);metadata=JSON.parse(lab.metadata());items[index]={id:`import-${index}`,local:false};updateLibrary();selectAsset(index);}
    error=null;notice(`Opened ${file.name}`);
  } catch(e) {notice(String(e),true);}
}
$('#open').onclick=()=>$('#file').click();$('#file').onchange=e=>{loadFile(e.target.files[0]);e.target.value='';};
let dragDepth=0;
document.addEventListener('dragenter',e=>{if(e.dataTransfer.types.includes('Files')){e.preventDefault();dragDepth++;$('#drop').hidden=false;}});
document.addEventListener('dragover',e=>e.preventDefault());document.addEventListener('dragleave',()=>{if(--dragDepth<=0){dragDepth=0;$('#drop').hidden=true;}});
document.addEventListener('drop',e=>{e.preventDefault();dragDepth=0;$('#drop').hidden=true;loadFile(e.dataTransfer.files[0]);});
function exportBytes(){
  // Save the selected Pokémon, including when it occupies the near battle side.
  lab.configure(JSON.stringify({...settings,asset:selected,reference:false}));
  try{return lab.export(selected);}finally{configure();}
}
$('#save').onclick=()=>{try{const url=URL.createObjectURL(new Blob([exportBytes()],{type:'application/zip'}));const a=document.createElement('a');a.href=url;a.download=(items[selected]?.id||'sprite')+'.spu';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}catch(e){notice(String(e),true);}};
async function createLabs(loaded) {
  items=loaded.map(x=>x.entry);
  const first=loaded[0].bytes,second=loaded[1]?.bytes||new Uint8Array();
  lab=await Lab.create($('#canvas'),first,second);
  original=await Lab.create($('#original'),first,second);
  for(const item of loaded.slice(2)){lab.load(item.bytes);original.load(item.bytes);}
  metadata=JSON.parse(lab.metadata());opponent=Math.min(1,metadata.assets.length-1);selected=0;
  updateLibrary();updateClips();sync();ready=true;$('#loading').hidden=true;$('#play').disabled=false;$('#save').disabled=false;resize();
}
window.spriteLab={get settings(){return structuredClone(settings);},get metadata(){return metadata;},get ready(){return ready;},get error(){return error;},get seconds(){return seconds;},get playing(){return playing;},get stats(){return {cpuMs:lastStats[0]||0,parts:lastStats[2]||0,lights:lastStats[3]||0,terrain:lastStats[4]||0};},get selected(){return selected;},set(values){Object.assign(settings,values);sync();},selectAsset,setLayout,applyScene,seek(t){seconds=t;playing=false;sync();},loadFile,exportBytes};
async function boot() {
  const query=new URLSearchParams(location.search);
  try {
    await init();
    const results=await Promise.allSettled(fixtures.map(async entry=>{const r=await fetch(entry.file);if(!r.ok)throw Error(`${entry.name} is unavailable`);return {entry:{...entry,local:true},bytes:new Uint8Array(await r.arrayBuffer())};}));
    const loaded=results.filter(r=>r.status==='fulfilled').map(r=>r.value);
    if(!loaded.length){$('#loading').textContent='No local Pokémon files found. Open an .spu file to compare it.';return;}
    await createLabs(loaded);
    const wanted=query.get('pokemon')||query.get('asset');const found=items.findIndex(i=>i.id===wanted);selected=found<0?0:found;
    const other=items.findIndex(i=>i.id===(query.get('opponent')||query.get('player')));opponent=other<0?(selected+1)%items.length:other;
    if([100,500,1000,5000,10000].includes(Number(query.get('count'))))count=+query.get('count');
    setLayout(query.get('demo')?'front':query.get('view')||'front');
    applyScene(({studio:0,cave:4,night:4,rain:6})[query.get('scene')]??0);
    if(results.some(r=>r.status==='rejected'))notice('Some local Pokémon files were unavailable.',true);
  } catch(e) {error=String(e);$('#loading').textContent=error;console.error(e);}
}
new ResizeObserver(resize).observe($('#comparison'));
requestAnimationFrame(frame);boot();
