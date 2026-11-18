import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

const binary = process.env.AGENT_BROWSER_BINARY ?? (process.platform === 'win32'
  ? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe') : 'agent-browser');
const session = `settings-exit-${process.pid}`;
const base = 'http://127.0.0.1:4197/';
function command(...args) {
  const run = spawnSync(binary, ['--session', session, '--json', ...args], { encoding: 'utf8', timeout: 40000 });
  assert.equal(run.status, 0, `${run.error ?? ''}\n${run.stdout}\n${run.stderr}`);
  const result = JSON.parse(run.stdout);
  assert.equal(result.success, true, run.stdout);
  return result.data;
}
const evaluate = source => command('eval', '-b', Buffer.from(source).toString('base64')).result;
const wait = source => command('wait', '--fn', source);
function click(name, scope = 'document') {
  evaluate(`(()=>{const b=[...${scope}.querySelectorAll('button')].find(x=>(x.getAttribute('aria-label')||x.textContent).trim()===${JSON.stringify(name)});if(!b)throw Error('Missing '+${JSON.stringify(name)});b.focus();b.click()})()`);
}
function selectTheme(label) {
  evaluate(`document.querySelector('[role="combobox"][aria-label="Theme"]').click()`);
  evaluate(`document.querySelector('[role="option"][data-label=${JSON.stringify(label)}]').click()`);
}
const tabs = () => [...new Set([...JSON.stringify(command('tab')).matchAll(/\bt\d+\b/g)].map(match => match[0]))];
const requests = () => evaluate('exitFixture.ledger.filter(x=>x.command.includes("shell_exit") || x.command==="get_shell_shutdown_status")');

try {
  command('open', `${base}?surface=control`);
  wait('!!document.querySelector(".theme-picker")');
  const [controlTab] = tabs();
  const before = evaluate('localStorage.getItem("jasonshell.themes.v1")');
  selectTheme('Base Light');
  wait('document.body.textContent.includes("Theme selection unavailable")');
  assert.equal(evaluate('localStorage.getItem("jasonshell.themes.v1")'), before);

  command('tab', 'new', base);
  wait('!!window.exitFixture && !!document.querySelector(".theme-editor")');
  const settingsTab = tabs().find(id => id !== controlTab);
  assert.ok(controlTab && settingsTab);
  command('tab', controlTab);
  selectTheme('Base Light');
  wait('JSON.parse(localStorage.getItem("jasonshell.themes.v1")||"null")?.selectedId==="base-light"');
  command('tab', settingsTab);

  click('Create copy');
  const name = 'document.querySelector(".theme-name input")';
  evaluate(`(()=>{const e=${name};e.value='Unsaved browser draft';e.dispatchEvent(new Event('input',{bubbles:true}));})()`);
  click('Exit JasonShell');
  wait('document.activeElement?.textContent.trim()==="Cancel"');
  assert.deepEqual(requests(), []);
  command('press', 'Escape');
  wait('!document.querySelector("[role=alertdialog]") && document.activeElement?.textContent.trim()==="Exit JasonShell"');
  evaluate(`document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',code:'Escape',repeat:true,bubbles:true,cancelable:true}))`);
  assert.equal(evaluate('exitFixture.ledger.some(x=>x.command==="hide_settings_panel")'), false);
  assert.equal(evaluate(`${name}.value`), 'Unsaved browser draft');
  assert.deepEqual(requests(), []);

  const stored = evaluate('localStorage.getItem("jasonshell.themes.v1")');
  click('Exit JasonShell');
  const dialog = 'document.querySelector("[role=alertdialog]")';
  wait(`!!${dialog}`);
  function requestThemeWhileExitOwnsFocus() {
    evaluate(`(()=>{window.selectionSeen=false;const c=new BroadcastChannel('jasonshell.themes.selection.v1');c.onmessage=({data})=>{if(data.type==='select'&&data.id==='monokai'){window.selectionSeen=true;c.close();}};})()`);
    command('tab', controlTab);
    selectTheme('Monokai');
    command('tab', settingsTab);
    wait('window.selectionSeen');
    assert.deepEqual(evaluate('[...document.querySelectorAll("[role=alertdialog]")].map(x=>x.querySelector("h2,h3")?.textContent.trim())'), ['Exit JasonShell?']);
  }
  requestThemeWhileExitOwnsFocus();
  click('Cancel', dialog);
  wait('document.querySelector("[role=alertdialog]")?.textContent.includes("Discard theme draft?")');
  click('Keep editing', dialog);
  wait(`!${dialog}`);
  assert.equal(evaluate(`${name}.value`), 'Unsaved browser draft');
  click('Exit JasonShell');
  wait(`!!${dialog}`);
  requestThemeWhileExitOwnsFocus();
  assert.match(evaluate(`${dialog}.textContent`), /unsaved[^.]*\b(lost|discarded)\b/i);
  evaluate(`(()=>{const b=[...${dialog}.querySelectorAll('button')].find(x=>/exit/i.test(x.textContent));b.click();b.click();})()`);
  wait('/exiting jasonshell/i.test(document.body.textContent)');
  assert.equal(evaluate('document.body.textContent.includes("Discard theme draft?")'), false);
  assert.deepEqual(requests(), [{ command: 'request_shell_exit', args: {} }]);
  assert.equal(evaluate('localStorage.getItem("jasonshell.themes.v1")'), stored);
  console.log('PASS isolated Settings exit, Escape, and cross-document theme authority (mocked native IPC)');
} finally {
  command('close');
}
