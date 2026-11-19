import { createNative, invoke as baselineInvoke } from '../seven-hotkeys/bridge';
import type { QuickCommandRunHistoryEntry } from '../../../src/lib/quickCommands';
export * from '../seven-hotkeys/bridge';

const overlay = 'quick-command-notification';
const panel = 'command-panel';
type Banner = { id: string; label: string; status: 'success' | 'failure' };
type Target = { commandId: string; runId: string };
export class Channel { onmessage = (_: unknown) => {}; }
export function createFixture() {
  return Object.assign(createNative(), {
    history: [] as QuickCommandRunHistoryEntry[], banner: null as Banner | null,
    requested: null as (Target & { notificationId: string }) | null, target: null as Target | null,
    panelVisible: false, serial: 0,
    channel: null as Channel | null, renderedId: null as string | null,
    finish(runId: string, announce = true, status: Banner['status'] = 'success') {
      const time = 1000 * ++this.serial;
      this.history.unshift({
        commandId: 'build', runId, startedAtEpochMs: time, finishedAtEpochMs: time + 200, processId: this.serial,
        exitCode: status === 'success' ? 0 : 1, running: false, stdout: '', stderr: '', stdoutTruncated: false, stderrTruncated: false,
        transcript: [{ kind: 'stdout', body: `${runId} output sentinel`, requestId: null, prompt: null,
          secret: false, redacted: false, sequence: 1, atEpochMs: time + 100, pending: false }],
      });
      if (announce) this.announce(runId, status);
    },
    announce(runId: string, status: Banner['status'] = 'success') {
      if (this.panelVisible || this.banner) throw new Error('Expected hidden panel and idle banner');
      this.banner = { id: `banner-${this.serial}`, label: 'Build', status };
      this.requested = { commandId: 'build', runId, notificationId: this.banner.id };
      this.channel?.onmessage(null);
    },
  });
}
export type Fixture = ReturnType<typeof createFixture>;
export const fixture = () => (parent as Window & { __sevenNative: Fixture }).__sevenNative;

export async function invoke(command: string, args: Record<string, any> = {}) {
  const n = fixture();
  const caller = new URLSearchParams(location.search).get('surface');
  const request = args.request ?? args;
  switch (command) {
    case 'load_shell_settings': return structuredClone(n.settings);
    case 'list_quick_command_history': return structuredClone(n.history.filter(run => !request.id || run.commandId === request.id));
    case 'get_quick_command_notification':
      if (caller !== overlay) throw new Error('Unexpected banner reader');
      if (request.channel) n.channel = request.channel;
      if (request.renderedId && request.renderedId === n.banner?.id) {
        if (!document.querySelector('button[aria-label*="Open Build output"]')) throw new Error('Banner acknowledged before DOM update');
        n.renderedId = request.renderedId;
      }
      return structuredClone(n.banner);
    case 'act_on_quick_command_notification': {
      if (caller !== overlay || n.banner?.id !== request.id || n.renderedId !== request.id || !['open', 'dismiss'].includes(request.action)) throw new Error('Invalid banner action');
      n.record('banner-action', { action: request.action, id: request.id });
      n.banner = null;
      n.channel?.onmessage(null);
      if (request.action === 'open') n.emit('top-bar', 'quick-command-notification:open-panel', { id: request.id });
      else n.requested = null;
      return;
    }
    case 'show_command_panel':
      if (caller !== 'top-bar') throw new Error('Panel must open through production TopBar');
      n.record('show', { args });
      if (request.notificationId) {
        if (request.notificationId !== n.requested?.notificationId) throw new Error('Invalid show token');
        n.target = { commandId: n.requested.commandId, runId: n.requested.runId };
        n.requested = null;
      }
      n.panelVisible = true;
      n.emit(panel, 'command-panel:open');
      if (n.target) n.emit(panel, 'command-panel:notification-target');
      return;
    case 'hide_command_panel':
      n.record('hide', { caller });
      n.panelVisible = false;
      n.emit('top-bar', 'command-panel:closed');
      return;
    case 'take_quick_command_notification_target': {
      if (caller !== panel) throw new Error('Unexpected target reader');
      const target = n.panelVisible ? n.target : null;
      if (target) n.target = null;
      return target;
    }
    default: return baselineInvoke(command, args);
  }
}
