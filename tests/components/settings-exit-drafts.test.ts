import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Settings from '../../src/components/SettingsPanelSurface.svelte';
import { defaultShellSettings } from '../../src/lib/settings';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/webviewWindow', () => ({ getCurrentWebviewWindow: () => ({ label: 'settings-panel', listen: tauriMocks().listen }) }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ label: 'settings-panel', listen: tauriMocks().listen }) }));
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes,no) => { resolve=yes; reject=no; }); return { promise,resolve,reject };
}
let captureBarrier: ReturnType<typeof deferred<any>> | null;
let saveBarrier: ReturnType<typeof deferred<any>> | null;
let committed: ReturnType<typeof defaultShellSettings>;
const calls = (name: string) => tauriMocks().invoke.mock.calls.filter(([command]) => command===name);
beforeEach(() => {
  localStorage.clear(); captureBarrier=null; saveBarrier=null;
  committed=defaultShellSettings();
  vi.spyOn(HTMLCanvasElement.prototype,'getContext').mockReturnValue(null);
  tauriMocks().listen.mockResolvedValue(vi.fn());
  tauriMocks().invoke.mockImplementation(async (command: string,args?: any) => {
    if(command==='load_shell_settings') return structuredClone(committed);
    if(command==='get_standard_hotkey_status') return {revision:1,bindings:[]};
    if(command==='get_speech_model_status') return {state:'missing',source:null,error:null};
    if(command==='begin_standard_hotkey_capture') return captureBarrier ? captureBarrier.promise : captureAck(args.request.revision);
    if(command==='renew_standard_hotkey_capture') return captureAck(args.request.revision);
    if(command==='end_standard_hotkey_capture') return;
    if(command==='save_standard_hotkeys') {
      if(saveBarrier) return saveBarrier.promise;
      throw Error('save barrier must be set by test');
    }
    throw Error(`Unexpected Settings exit-draft native action: ${command}`);
  });
});
function captureAck(revision: number) { return {leaseId:'exit-draft-lease',revision,sequence:0,expiresInMs:2000,registeredBindings:Object.values(committed.hotkeys)}; }
async function mount() {
  const view=render(Settings);
  const region=within(view.getByRole('region',{name:'Keyboard shortcuts'}));
  await waitFor(()=>expect((region.getByRole('button',{name:'Capture Search shortcut'}) as HTMLButtonElement).disabled).toBe(false));
  return {view,region};
}
async function stage(region: ReturnType<typeof within>) {
  const button=region.getByRole('button',{name:'Capture Search shortcut'}); button.focus(); await fireEvent.click(button);
  await waitFor(()=>expect(region.getByRole('status').textContent).toContain('Listening for Search'));
  await fireEvent.keyDown(button,{key:'x',code:'KeyX',altKey:true});
  await waitFor(()=>expect(button.textContent).toContain('Alt+X'));
  return button;
}
async function cancelExit(view: ReturnType<typeof render>) {
  await fireEvent.click(view.getByRole('button',{name:'Exit JasonShell',exact:true}));
  const dialog=await view.findByRole('alertdialog');
  await fireEvent.click(within(dialog).getByRole('button',{name:'Cancel',exact:true}));
}

describe('Settings shortcut editing during local exit confirmation',()=>{
  it('local exit cancellation neither retires nor commits pending capture',async()=>{
    captureBarrier=deferred(); const {view,region}=await mount();
    await fireEvent.click(region.getByRole('button',{name:'Capture Search shortcut'}));
    await waitFor(()=>expect(calls('begin_standard_hotkey_capture')).toHaveLength(1));
    await cancelExit(view);
    expect(calls('request_shell_exit')).toHaveLength(0); expect(calls('end_standard_hotkey_capture')).toHaveLength(0); expect(calls('save_standard_hotkeys')).toHaveLength(0);
    const revision=(calls('begin_standard_hotkey_capture')[0][1] as any).request.revision;
    captureBarrier.resolve(captureAck(revision));
    await waitFor(()=>expect(region.getByRole('button',{name:'Capture Search shortcut'}).getAttribute('aria-pressed')).toBe('true'));
  });

  it('staged binding remains dirty and exact after cancelled exit without implicit shortcut save',async()=>{
    const {view,region}=await mount(); const button=await stage(region);
    const before=button.textContent; await cancelExit(view);
    expect(button.textContent).toBe(before); expect(calls('save_standard_hotkeys')).toHaveLength(0); expect(calls('request_shell_exit')).toHaveLength(0);
  });

  it('outstanding Save remains busy until actual commit receipt; Cancel exit does not duplicate Save',async()=>{
    const {view,region}=await mount(); await stage(region); saveBarrier=deferred();
    await fireEvent.click(region.getByRole('button',{name:'Save shortcuts'}));
    await waitFor(()=>expect(calls('save_standard_hotkeys')).toHaveLength(1));
    expect((region.getByRole('button',{name:/sav/i}) as HTMLButtonElement).disabled).toBe(true);
    await cancelExit(view); expect(calls('save_standard_hotkeys')).toHaveLength(1);
    const hotkeys=(calls('save_standard_hotkeys')[0][1] as any).request.hotkeys;
    committed={...committed,hotkeys};
    saveBarrier.resolve({committed:true,settings:structuredClone(committed),status:{revision:2,bindings:[]},statusError:null});
    await waitFor(()=>expect(region.getByRole('button',{name:'Save shortcuts'})).toHaveProperty('disabled',false));
    expect(region.getByRole('button',{name:'Capture Search shortcut'}).textContent).toContain('Alt+X');
  });

  it('rejected Save releases busy receipt but retains staged dirty binding for explicit resolution',async()=>{
    const {view,region}=await mount(); await stage(region); saveBarrier=deferred();
    await fireEvent.click(region.getByRole('button',{name:'Save shortcuts'}));
    await waitFor(()=>expect(calls('save_standard_hotkeys')).toHaveLength(1)); saveBarrier.reject(Error('registration failed'));
    await waitFor(()=>expect(region.getByRole('button',{name:'Save shortcuts'})).toHaveProperty('disabled',false));
    await cancelExit(view); expect(region.getByRole('button',{name:'Capture Search shortcut'}).textContent).toContain('Alt+X');
    expect(calls('save_standard_hotkeys')).toHaveLength(1); expect(calls('request_shell_exit')).toHaveLength(0);
  });
});
