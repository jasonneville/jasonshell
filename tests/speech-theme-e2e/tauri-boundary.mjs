// Test-only native boundary. Theme modules and browser channels are NOT mocked.
const handlers = new Map();
let state = { status: 'recording', nonce: 7 };
let voiceTimer;
window.__speechThemeVoice = (level) => {
  clearInterval(voiceTimer);
  const send = () => {
    for (const handler of handlers.get('speech:voice-level') ?? []) handler({ payload: { nonce: 7, level } });
  };
  send();
  if (level > 0) voiceTimer = setInterval(send, 50);
};
window.__speechThemeStatus = (status) => {
  clearInterval(voiceTimer);
  state = { status, nonce: status === 'recording' ? 7 : null };
  for (const handler of handlers.get('speech:status-changed') ?? []) handler({ payload: state });
};
export async function listen(event, handler) {
  if (!handlers.has(event)) handlers.set(event, new Set());
  handlers.get(event).add(handler);
  return () => handlers.get(event).delete(handler);
}
export async function emitTo() { /* No native event destinations in browser test. */ }
export async function emit() { /* No native event publication in browser test. */ }
export function getCurrentWindow() {
  return { label: new URL(location.href).searchParams.get('surface'),
    show: async () => {}, hide: async () => {} };
}
export async function invoke(command) {
  if (command === 'get_speech_status') return state;
  if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
  if (command === 'load_shell_settings') {
    const { defaultShellSettings } = await import('../../src/lib/settings');
    return defaultShellSettings();
  }
  throw new Error(`Unexpected native command in theme E2E: ${command}`);
}
export function convertFileSrc(path) { return path; }
