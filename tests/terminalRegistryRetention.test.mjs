import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import test from 'node:test';

const source = readFileSync(new URL('../src-tauri/src/stack_popup/terminal.rs', import.meta.url), 'utf8');
function block(marker) {
  const start = source.indexOf(marker);
  assert.notEqual(start, -1, `Missing production declaration ${marker}`);
  const open = source.indexOf('{', start);
  let depth = 1;
  let end = open + 1;
  // Scoped Rust declarations below have no brace-bearing strings/comments.
  while (depth && end < source.length) {
    if (source[end] === '{') depth += 1;
    if (source[end] === '}') depth -= 1;
    end += 1;
  }
  assert.equal(depth, 0);
  return source.slice(start, end);
}

test('actual production registry churn removes obsolete ownership, preserves targets, and retains no tombstones', (t) => {
  // Isolate registry ownership from ConPTY/Tauri/ONNX linking. Session is a
  // drop-observable value double; every registry operation is verbatim source.
  const methods = ['fn insert(', 'fn request_stop(', 'fn session_mut(', 'fn remove(', 'fn list_by_target(', 'fn request_stop_by_target('].map(block).join('\n');
  const tombstoneCount = /stop_requested\s*:/.test(block('pub(crate) struct StackTerminalRegistry')) ? 'registry.stop_requested.len()' : '0usize';
  const productionDeclarations = [block('pub(crate) struct StackTerminalRegistry'), block('impl Default for StackTerminalRegistry'), methods, block('pub(crate) fn validate_stack_terminal_session_id('), block('fn authorize_stack_terminal_session_target(')].join('\n');
  t.diagnostic(`production_declarations_sha256=${createHash('sha256').update(productionDeclarations).digest('hex')}`);
  const fixture = `
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
const MAX_STACK_TERMINAL_SESSION_ID_LEN: usize = 48;
const STACK_TERMINAL_SESSION_PREFIX: &str = "stack-term-";
struct StackTerminalSession { id: String, target_label: String, drops: Arc<AtomicUsize> }
struct StackTerminalSessionSnapshot { created_at: u64 }
impl StackTerminalSession { fn snapshot(&self) -> StackTerminalSessionSnapshot { StackTerminalSessionSnapshot { created_at: 0 } } }
impl Drop for StackTerminalSession { fn drop(&mut self) { self.drops.fetch_add(1, Ordering::SeqCst); } }
${block('pub(crate) struct StackTerminalRegistry')}
${block('impl Default for StackTerminalRegistry')}
impl StackTerminalRegistry { ${methods} }
${block('pub(crate) fn validate_stack_terminal_session_id(')}
${block('fn authorize_stack_terminal_session_target(')}
fn session(id: &str, target: &str, drops: &Arc<AtomicUsize>) -> StackTerminalSession {
  StackTerminalSession { id: id.into(), target_label: target.into(), drops: Arc::clone(drops) }
}
fn main() {
  let drops = Arc::new(AtomicUsize::new(0));
  let mut registry = StackTerminalRegistry::default();
  for i in 0..10000 {
    let id = format!("stack-term-{i}");
    registry.insert(session(&id, "terminal-panel", &drops));
    let removed = registry.request_stop(&id).unwrap().expect("stop transfers owned session");
    assert!(registry.session_mut(&id).is_err(), "stopped session cannot be polled or resurrected");
    assert!(registry.request_stop(&id).unwrap().is_none(), "repeat stop must not create ownership");
    drop(removed);
  }
  assert_eq!(drops.load(Ordering::SeqCst), 10000);
  assert_eq!(registry.sessions.len(), 0);
  let retained = ${tombstoneCount};
  println!("churn=10000 active={} retained_tombstones={} dropped={}", registry.sessions.len(), retained, drops.load(Ordering::SeqCst));
  registry.insert(session("stack-term-panel", "terminal-panel", &drops));
  registry.insert(session("stack-term-stack", "stack-popup", &drops));
  let stopped = registry.request_stop_by_target("terminal-panel");
  assert_eq!(stopped.len(), 1);
  assert!(registry.session_mut("stack-term-panel").is_err());
  assert!(registry.session_mut("stack-term-stack").is_ok());
  assert_eq!(drops.load(Ordering::SeqCst), 10000, "registry must transfer cleanup ownership, not destroy it early");
  drop(stopped);
  assert_eq!(drops.load(Ordering::SeqCst), 10001);
  assert!(registry.request_stop("../invalid").is_err());
  assert!(authorize_stack_terminal_session_target("terminal-panel", "stack-popup", "poll_stack_terminal_session").is_err());
  assert!(authorize_stack_terminal_session_target("stack-popup", "terminal-panel", "stop_stack_terminal").is_err());
  assert!(authorize_stack_terminal_session_target("terminal-panel", "terminal-panel", "poll_stack_terminal_session").is_ok());
  drop(registry.remove("stack-term-stack").unwrap());
  assert_eq!(drops.load(Ordering::SeqCst), 10002);
  assert_eq!(retained, 0, "obsolete production stop tombstones retained after churn");
  assert_eq!(${tombstoneCount}, 0, "target-scoped stop retained obsolete tombstones");
}
`;
  const directory = mkdtempSync(join(process.env.LOCALAPPDATA ? join(process.env.LOCALAPPDATA, 'Temp', 'opencode') : tmpdir(), 'terminal-registry-'));
  try {
    const input = join(directory, 'registry.rs');
    const executable = join(directory, process.platform === 'win32' ? 'registry.exe' : 'registry');
    writeFileSync(input, fixture);
    const compile = spawnSync('rustc', ['--edition=2021', '-Awarnings', input, '-o', executable], { encoding: 'utf8', timeout: 120000 });
    assert.equal(compile.status, 0, compile.stderr || String(compile.error));
    const run = spawnSync(executable, [], { encoding: 'utf8', timeout: 30000 });
    t.diagnostic(run.stdout.trim());
    assert.equal(run.status, 0, run.stderr || String(run.error));
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
