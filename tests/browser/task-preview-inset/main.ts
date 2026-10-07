import '../../../src/app.css';
import { mount, tick } from 'svelte';
import TaskPreviewSurface from '../../../src/components/TaskPreviewSurface.svelte';
import { handlers, publish, ledger } from './bridge';
const image = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><rect width="640" height="360" fill="#507396"/><text x="40" y="80" fill="white">Captured application</text></svg>');
const icon = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18"><rect width="18" height="18" fill="#8abaeb"/></svg>');
mount(TaskPreviewSurface, { target: document.getElementById('surface')! });
const failures: string[] = [];
const evidence: unknown[] = [];
function check(ok: boolean, message: string) { if (!ok) failures.push(message); }
function rect(el: Element) { const r = el.getBoundingClientRect(); return { left:r.left, top:r.top, right:r.right, bottom:r.bottom, width:r.width, height:r.height }; }
async function scenario(theme = 'base-dark', source = 'captured-image', long = false, hasIcon = true) {
  document.documentElement.dataset.theme = theme;
  await publish({ hwnd:'1234', title:long ? 'Exceptionally long document title '.repeat(30) : 'Editor document', processName:long ? 'Long process '.repeat(30) : 'editor.exe', iconDataUrl:hasIcon ? icon : '', isMinimized:false, previewSource:source, imageDataUrl:source === 'captured-image' ? image : null });
  await tick();
  const activate = document.querySelector<HTMLElement>('[aria-label^="Activate "]')!;
  const close = document.querySelector<HTMLElement>('[aria-label="Close previewed window"]')!;
  const frame = document.querySelector<HTMLElement>('.preview-frame, .preview-empty')!;
  const title = document.querySelector<HTMLElement>('.preview-title')!;
  const style = getComputedStyle(activate), fs = getComputedStyle(frame);
  const a = rect(activate), c = rect(close), f = rect(frame), t = rect(title);
  const label = `${theme}/${source}/${long ? 'long' : 'normal'}/${hasIcon ? 'icon' : 'missing'}`;
  check(style.paddingTop === '10px' && style.borderTopWidth === '1px', `${label}: border1 padding10`);
  check(style.backgroundImage !== 'none', `${label}: theme gradient`);
  check(parseFloat(style.borderRadius) >= 7 && parseFloat(style.borderRadius) <= 9, `${label}: soft8 corners`);
  check(getComputedStyle(close).backgroundColor !== 'rgba(0, 0, 0, 0)', `${label}: filled close`);
  check(c.left >= a.left && c.right <= a.right && c.top >= a.top && c.bottom <= a.bottom, `${label}: close containment`);
  check(t.right <= c.left, `${label}: caption/close nonoverlap`);
  if (long) check(title.scrollWidth > title.clientWidth && getComputedStyle(title).textOverflow === 'ellipsis', `${label}: actual title truncation`);
  const appIcon = document.querySelector<HTMLImageElement>('.preview-header img');
  check(Boolean(appIcon) === hasIcon, `${label}: decorative icon state`);
  if (appIcon) check(rect(appIcon).width === 18 && rect(appIcon).height === 18 && appIcon.alt === '', `${label}: decorative18 icon`);
  const content = { left:f.left-a.left+parseFloat(fs.borderLeftWidth), top:f.top-a.top+parseFloat(fs.borderTopWidth), right:f.right-a.left-parseFloat(fs.borderRightWidth), bottom:f.bottom-a.top-parseFloat(fs.borderBottomWidth) };
  for (const [key, expected] of Object.entries({ left:12, top:48, right:320, bottom:216 })) check(Math.abs(content[key as keyof typeof content]-expected) < 0.1, `${label}: content ${key} expected ${expected}, got ${content[key as keyof typeof content]}`);
  if (source === 'native-dwm-thumbnail') check(fs.backgroundColor === 'rgba(0, 0, 0, 0)' && fs.borderTopColor !== 'rgba(0, 0, 0, 0)', `${label}: native transparent interior visible perimeter`);
  activate.focus(); check(document.activeElement === activate, `${label}: activation focus`);
  close.focus(); check(document.activeElement === close && !activate.contains(close), `${label}: independent close focus`);
  evidence.push({ label, content, close:c, title:t, background:style.backgroundImage, shadow:style.boxShadow });
}
async function run() {
  failures.length = 0; evidence.length = 0;
  for (const theme of ['base-dark','base-light']) for (const source of ['captured-image','native-dwm-thumbnail','unavailable']) for (const long of [false,true]) for (const hasIcon of [false,true]) await scenario(theme,source,long,hasIcon);
  await scenario();
  const broken = document.querySelector('.preview-header img');
  broken?.dispatchEvent(new Event('error')); await tick();
  check(!document.querySelector('.preview-header img'), 'broken icon omitted');
  const replacementIcon = icon + '%20';
  await publish({ hwnd:'1234', title:'Replacement icon', processName:'editor.exe', iconDataUrl:replacementIcon, isMinimized:false, previewSource:'captured-image', imageDataUrl:image });
  await tick();
  check(document.querySelector('.preview-header img')?.getAttribute('src') === replacementIcon, 'new icon URL recovers after broken icon');
  await publish({ hwnd:'1234', title:'Blank icon', processName:'editor.exe', iconDataUrl:'   ', isMinimized:false, previewSource:'unavailable', imageDataUrl:null });
  await tick();
  check(!document.querySelector('.preview-header img'), 'whitespace icon omitted');
  const result = { failures, evidence, scales:[1,1.25,1.5,2].map(scale => ({ scale, expectedPhysical:{ left:12*scale, top:48*scale, right:320*scale, bottom:216*scale } })), nativeCompositing:'NOT TESTED; mocked IPC only' };
  document.getElementById('results')!.textContent = JSON.stringify(result,null,2);
  return result;
}
Object.assign(window,{ previewFixture:{ run,scenario,ledger } });
await tick();
if (!handlers.has('task-preview:update')) throw new Error('Preview listener missing');
Object.assign(window,{ previewReady:true });
await scenario();
