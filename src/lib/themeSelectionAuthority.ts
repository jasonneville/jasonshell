/** Cross-document selection requests, never cross-document storage writes.
 * Only the mounted Settings theme editor installs the authority. The offer
 * identifies that mount, so a replacement cannot accept a stale selection.
 * There is no foreign-write fallback.
 */

export const THEME_SELECTION_CHANNEL = 'jasonshell.themes.selection.v1';
export const THEME_SELECTION_DEADLINE_MS = 30_000;
export const THEME_SELECTION_DISCOVERY_MS = 3_000;
const MAX_REQUESTS = 128;
type Handler = (id: string, expiresAt: number) => Promise<boolean>;
type Authority = { registrationId: string; select: Handler };
let authority: Authority | null = null;
const validNonce = (value: unknown): value is string => typeof value === 'string' && /^[\da-f-]{32,36}$/.test(value);
const validId = (value: unknown): value is string => typeof value === 'string' && /^[\w-]{1,120}$/.test(value);
const unavailable = () => new Error('Theme selection unavailable. Open Settings and resolve any theme draft, then retry.');

export function assertThemePersistenceAuthority(): void {
  if (!authority) throw unavailable();
}

export function installThemeSelectionAuthority(registrationId: string, select: Handler): () => void {
  if (!/^[\da-f]{32}$/.test(registrationId) || authority) throw unavailable();
  const current: Authority = { registrationId, select };
  const channel = typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel(THEME_SELECTION_CHANNEL);
  const requests = new Map<string, { id: string; expiresAt: number; state: 'offered' | 'pending' | 'done'; accepted: boolean; unavailable: boolean }>();
  const send = (packet: object) => {
    try { channel?.postMessage(packet); } catch { /* Requester times out; never fall back to a foreign write. */ }
  };
  authority = current;
  if (channel) channel.onmessage = ({ data: value }: MessageEvent<unknown>) => {
    if (authority !== current || !value || typeof value !== 'object') return;
    const packet = value as Record<string, unknown>;
    if (packet.version !== 1 || !validNonce(packet.requestId) || !validId(packet.id)
      || !Number.isSafeInteger(packet.expiresAt)) return;
    const expiresAt = packet.expiresAt as number;
    const now = Date.now();
    if (expiresAt <= now || expiresAt > now + THEME_SELECTION_DEADLINE_MS) return;
    for (const [key, request] of requests) if (request.expiresAt <= now) requests.delete(key);
    const previous = requests.get(packet.requestId);
    if (packet.type === 'discover') {
      if (previous && (previous.id !== packet.id || previous.expiresAt !== expiresAt)) return;
      if (!previous) {
        // Never evict an unexpired receipt: replay must not produce another write.
        if (requests.size >= MAX_REQUESTS) return;
        requests.set(packet.requestId, { id: packet.id, expiresAt, state: 'offered', accepted: false, unavailable: false });
      }
      send({ version: 1, type: 'offer', requestId: packet.requestId, registrationId });
      return;
    }
    if (packet.type !== 'select' || packet.registrationId !== registrationId || !previous
      || previous.id !== packet.id || previous.expiresAt !== expiresAt) return;
    const reply = () => {
      if (authority === current && Date.now() < expiresAt) send({
        version: 1, type: 'result', requestId: packet.requestId, registrationId, accepted: previous.accepted, unavailable: previous.unavailable
      });
    };
    if (previous.state === 'done') { reply(); return; }
    if (previous.state === 'pending') return;
    previous.state = 'pending';
    // Recheck ownership and deadline before dispatching the selection.
    void Promise.resolve().then(() => {
      if (authority !== current || Date.now() >= expiresAt) throw unavailable();
      return current.select(previous.id, expiresAt);
    }).then((accepted) => { previous.accepted = accepted; }, () => { previous.accepted = false; previous.unavailable = true; })
      .finally(() => { previous.state = 'done'; reply(); });
  };
  return () => {
    if (authority === current) authority = null;
    channel?.close();
    requests.clear();
  };
}

export async function requestThemeSelectionThroughAuthority(id: string): Promise<boolean> {
  if (!validId(id)) throw unavailable();
  const expiresAt = Date.now() + THEME_SELECTION_DEADLINE_MS;
  if (authority) return authority.select(id, expiresAt);
  if (typeof BroadcastChannel === 'undefined') throw unavailable();
  return new Promise<boolean>((resolve, reject) => {
    const channel = new BroadcastChannel(THEME_SELECTION_CHANNEL);
    const requestId = crypto.randomUUID();
    let registrationId = '';
    let settled = false;
    const finish = (accepted: boolean, failed = false) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer); channel.onmessage = null; channel.close();
      if (failed) reject(unavailable()); else resolve(accepted);
    };
    let timer = setTimeout(() => finish(false, true), THEME_SELECTION_DISCOVERY_MS);
    channel.onmessage = ({ data: value }: MessageEvent<unknown>) => {
      if (settled || !value || typeof value !== 'object' || Date.now() >= expiresAt) return;
      const packet = value as Record<string, unknown>;
      if (packet.version !== 1 || packet.requestId !== requestId || !validNonce(packet.registrationId)) return;
      if (packet.type === 'offer' && !registrationId) {
        registrationId = packet.registrationId;
        clearTimeout(timer);
        timer = setTimeout(() => finish(false, true), Math.max(0, expiresAt - Date.now()));
        try { channel.postMessage({ version: 1, type: 'select', requestId, registrationId, id, expiresAt }); }
        catch { finish(false, true); }
      } else if (packet.type === 'result' && registrationId === packet.registrationId && typeof packet.accepted === 'boolean' && typeof packet.unavailable === 'boolean') finish(packet.accepted, packet.unavailable);
    };
    try { channel.postMessage({ version: 1, type: 'discover', requestId, id, expiresAt }); }
    catch { finish(false, true); }
  });
}
