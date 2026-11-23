import assert from 'node:assert/strict';
import test from 'node:test';
import { functionBody, productionPart, readRust } from './support/rustSource.mjs';

// SEC-03 / RS-14 source contracts for call sites that cannot be unit-driven
// without spawning git. Pure arg/cap behaviour lives in
// src-tauri/src/stack_popup/git_tier0_tests.rs. All matches run on
// comment-stripped production code.
const gitStatus = productionPart(readRust('src-tauri/src/stack_popup/git_status.rs'));
const devGit = productionPart(readRust('src-tauri/src/dev_tools/git_status.rs'));
const terminal = productionPart(readRust('src-tauri/src/stack_popup/terminal.rs'));

test('run_git_command builds argv through hardened_git_args and the shared stdout cap', () => {
  const body = functionBody(gitStatus, 'run_git_command');
  assert.ok(body, 'run_git_command body not found');
  assert.match(body, /hardened_git_args\(/);
  assert.match(body, /stdout_cap:\s*GIT_STDOUT_CAP_BYTES/);
  assert.ok(!/stdout_cap:\s*64 \* 1024/.test(body), 'stdout cap must no longer be 64 KiB');
});

test('git_stdout_bytes surfaces truncation as an error', () => {
  const body = functionBody(gitStatus, 'git_stdout_bytes');
  assert.ok(body, 'git_stdout_bytes body not found');
  assert.match(body, /ensure_stdout_complete\(/);
});

test('trusted git discovery goes through the pure git_candidate_paths seam', () => {
  const body = functionBody(gitStatus, 'git_candidates');
  assert.ok(body, 'git_candidates body not found');
  assert.match(body, /git_candidate_paths\(/);
});

test('dev_tools git status runs git only through the hardened shared runner (real call, not comment text)', () => {
  assert.ok(!/Command::new\("git"\)/.test(devGit), 'bare PATH git lookup');
  assert.match(devGit, /\b(?:workspace_git_stdout|run_git_command)\(\s*&?\w+/, 'expected a real call to the hardened runner');
  assert.match(devGit, /trusted_git_path\(\)/, 'dev_tools must also resolve the trusted git path');
  const status = functionBody(devGit, 'get_workspace_git_status');
  assert.ok(status, 'get_workspace_git_status body not found');
  assert.match(status, /workspace_git_stdout\(|run_git_command\(/);
  const gitDir = functionBody(devGit, 'resolve_git_dir');
  assert.ok(gitDir, 'resolve_git_dir body not found');
  assert.match(gitDir, /workspace_git_stdout\(|run_git_command\(/);
});

test('terminal prompt integration never runs bare git subcommands', () => {
  assert.ok(
    !/\bgit (rev-parse|symbolic-ref|status)\b/.test(terminal),
    'prompt script must call git with -c core.fsmonitor=false (e.g. via a hardened args splat)',
  );
  assert.match(terminal, /core\.fsmonitor=false/);
});
