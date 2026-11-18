import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installShellThemeSelectionAuthority, requestShellThemeSelection } from '../../src/lib/themes';
import { THEME_SELECTION_CHANNEL, THEME_SELECTION_DISCOVERY_MS, THEME_SELECTION_DEADLINE_MS } from '../../src/lib/themeSelectionAuthority';

// Only transport is replaced. Authority validation,
// canonical theme writer, storage envelope and correlation code are production.
class Channel {
  static channels=new Set<Channel>();
  onmessage:((event:{data:any})=>void)|null=null;
  constructor(readonly name:string){Channel.channels.add(this);}
  postMessage(data:any){for(const peer of Channel.channels)if(peer!==this&&peer.name===this.name)queueMicrotask(()=>peer.onmessage?.({data:structuredClone(data)}));}
  close(){Channel.channels.delete(this);this.onmessage=null;}
}
const flush=async()=>{for(let i=0;i<20;i++)await Promise.resolve();};
const key='jasonshell.themes.v1';
let id:string;
let dispose:(()=>void)|undefined;
let packets:any[];
let peer:Channel;
function discover(requestId='a'.repeat(32),theme='base-light',expiresAt=Date.now()+THEME_SELECTION_DEADLINE_MS){const packet={version:1,type:'discover',requestId,id:theme,expiresAt};peer.postMessage(packet);return packet;}
async function select(packet:ReturnType<typeof discover>,registrationId=id){peer.postMessage({...packet,type:'select',registrationId});await flush();}
function install(){dispose=installShellThemeSelectionAuthority();}
beforeEach(()=>{localStorage.clear();packets=[];id='';vi.stubGlobal('BroadcastChannel',Channel);peer=new Channel(THEME_SELECTION_CHANNEL);peer.onmessage=e=>{packets.push(e.data);if(e.data.type==='offer')id=e.data.registrationId;};});
afterEach(()=>{dispose?.();dispose=undefined;for(const channel of [...Channel.channels])channel.close();vi.useRealTimers();vi.restoreAllMocks();});

describe('Settings-owned canonical theme selection authority',()=>{
  it('normal foreign discovery/select invokes actual canonical persistence once; duplicate/replay returns cached receipt',async()=>{
    install();const writes=vi.spyOn(Storage.prototype,'setItem');
    const packet=discover();await flush();expect(packets.at(-1)).toMatchObject({type:'offer',requestId:packet.requestId,registrationId:id});
    await select(packet);const saved=localStorage.getItem(key);expect(JSON.parse(saved!).selectedId).toBe('base-light');
    expect(packets.at(-1)).toMatchObject({type:'result',accepted:true,unavailable:false,registrationId:id});
    await select(packet);expect(localStorage.getItem(key)).toBe(saved);
    expect(writes.mock.calls.filter(([name])=>name===key)).toHaveLength(1);
    peer.postMessage({...packet,id:'monokai'});await select({...packet,id:'monokai'});expect(localStorage.getItem(key)).toBe(saved);
  });

  it('absent authority rejects bounded discovery without a foreign localStorage fallback',async()=>{
    vi.useFakeTimers();const requested=requestShellThemeSelection('base-light');const assertion=expect(requested).rejects.toThrow(/unavailable/);
    await vi.advanceTimersByTimeAsync(THEME_SELECTION_DISCOVERY_MS+1);await assertion;
    expect(localStorage.getItem(key)).toBeNull();expect(localStorage.getItem('jasonshell.theme')).toBeNull();
  });

  it('replacement authority rejects old registration and requires a fresh correlated discovery',async()=>{
    install();const old=discover();await flush();const oldId=id;dispose!();install();
    const fresh=discover('b'.repeat(32));await flush();expect(id).not.toBe(oldId);
    await select(old,oldId);await select(old,id);expect(localStorage.getItem(key)).toBeNull();
    await select(fresh,id);
    expect(JSON.parse(localStorage.getItem(key)!).selectedId).toBe('base-light');
  });

  it('expired, mismatched-deadline and unknown-request selects never write',async()=>{
    install();const packet=discover();await flush();
    await select({...packet,expiresAt:packet.expiresAt-1});
    await select({...packet,requestId:'c'.repeat(32)});
    const expired=discover('d'.repeat(32),'monokai',Date.now()-1);await flush();await select(expired);
    expect(localStorage.getItem(key)).toBeNull();
    await select(packet);expect(JSON.parse(localStorage.getItem(key)!).selectedId).toBe('base-light');
  });

  it('128 unexpired receipts stay retained under saturation; first accepted replay cannot rewrite storage',async()=>{
    install();const writes=vi.spyOn(Storage.prototype,'setItem');
    const first=discover('e'.repeat(32));await flush();await select(first);const saved=localStorage.getItem(key);
    for(let n=1;n<128;n++)discover(n.toString(16).padStart(32,'0'));
    await flush();const offers=packets.filter(p=>p.type==='offer').length;
    discover('f'.repeat(32),'monokai');await flush();expect(packets.filter(p=>p.type==='offer')).toHaveLength(offers);
    await select(first);expect(localStorage.getItem(key)).toBe(saved);expect(writes.mock.calls.filter(([name])=>name===key)).toHaveLength(1);
  });
});
