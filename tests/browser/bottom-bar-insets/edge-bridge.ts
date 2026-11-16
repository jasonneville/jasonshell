import { defaultShellSettings } from '../../../src/lib/settings';
import { taskGalleryLayout } from '../../../src/lib/taskGalleryLayout';
type Handler = (event: { payload: any }) => unknown;
export const ledger: { name: string; args: any }[] = [];
const handlers = new Map<string, Set<Handler>>();
const params = new URLSearchParams(location.search);
export const settings = defaultShellSettings();
settings.ui.bottomBarHeightLogical = Number(params.get('height') ?? 32.4);
settings.ui.lockBottomBarHeight = params.get('unlocked') !== '1';
const iconDataUrl = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#82b5ef"/></svg>');
export const rows = Array.from({ length: 6 }, (_, i) => ({ hwnd: String(501 + i), processId: i ? 502 : 501,
  processName: i ? 'Grouped editor' : 'Direct editor', title: i ? `Grouped document ${i}` : 'Direct — a very long document title for edge hover ticker continuity regression',
  iconDataUrl, isActive: i === 0, isMinimized: i === 1, activityState: i === 2 ? 'busy' : 'idle',
  attentionState: i === 0 ? 'requested' : 'idle', toastCount: i === 0 ? 1 : 0 }));
let sequence = 0;
let releaseActivation: (() => void) | null = null;
export const control = { deferActivation: false, release: () => { releaseActivation?.(); releaseActivation = null; } };
export async function listen(name: string, handler: Handler) {
  const set = handlers.get(name) ?? new Set<Handler>(); set.add(handler); handlers.set(name, set);
  return () => { set.delete(handler); };
}
export async function publish(name: string, payload: unknown) {
  await Promise.all([...handlers.get(name) ?? []].map(handler => handler({ payload })));
}
export async function emit(name: string, payload: unknown) { await publish(name, payload); }
export async function emitTo(_target: unknown, name: string, payload: unknown) { await publish(name, payload); }
export function getCurrentWindow() { return { label: 'bottom-bar', listen }; }
export async function invoke<T>(name: string, args: any = {}): Promise<T> {
  ledger.push({ name, args });
  switch (name) {
    case 'load_shell_settings': return settings as T;
    case 'list_pinned_taskbar_apps': return [] as T;
    case 'list_open_task_windows': return rows as T;
    case 'allocate_task_preview_request_id': return ++sequence as T;
    case 'show_task_gallery': return taskGalleryLayout(args.args.windows.length, innerWidth, innerHeight) as T;
    case 'activate_task_window':
      if (control.deferActivation) await new Promise<void>(resolve => { releaseActivation = resolve; });
      return undefined as T;
    case 'save_shell_bar_height': settings.ui.bottomBarHeightLogical = args.request.heightLogical; return settings as T;
    case 'resize_shell_bar':
      document.getElementById('host')!.style.height = `${args.request.heightLogical}px`; return args.request as T;
    case 'hide_quick_launch_panel':
    case 'show_quick_launch_panel':
    case 'show_process_manager':
    case 'show_task_window_preview':
    case 'hide_task_window_preview':
    case 'hide_task_gallery':
    case 'request_taskbar_windows_refresh':
    case 'show_context_menu_overlay':
    case 'hide_context_menu_overlay':
    case 'report_shell_surface_runtime_metrics': return undefined as T;
    default: throw new Error(`Bottom-edge fixture refuses unexpected IPC: ${name}`);
  }
}
