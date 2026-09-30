import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import { defaultShellSettings, saveShellSettings } from '../dist-tests/lib/settings.js';

const originalTauriInternals = globalThis.__TAURI_INTERNALS__;
const originalWindow = globalThis.window;

afterEach(() => {
  if (originalTauriInternals === undefined) {
    delete globalThis.__TAURI_INTERNALS__;
  } else {
    globalThis.__TAURI_INTERNALS__ = originalTauriInternals;
  }
  if (originalWindow === undefined) {
    delete globalThis.window;
  } else {
    globalThis.window = originalWindow;
  }
});

function installInvokeRecorder() {
  const calls = [];
  globalThis.__TAURI_INTERNALS__ = {
    invoke: async (command, args) => {
      calls.push({ command, args });
      return args.settings;
    }
  };
  globalThis.window = {
    __TAURI_INTERNALS__: globalThis.__TAURI_INTERNALS__,
    dispatchEvent: () => true,
    localStorage: {
      setItem: () => undefined,
      removeItem: () => undefined
    }
  };
  return calls;
}

function shellSettingsWithTranscriptSecret() {
  return {
    ...defaultShellSettings(),
    quickCommands: {
      ...defaultShellSettings().quickCommands,
      history: [
        {
          runId: 'run-1',
          commandId: 'command-1',
          startedAtEpochMs: 1,
          finishedAtEpochMs: 2,
          processId: 100,
          exitCode: 0,
          stdout: '',
          stderr: '',
          stdoutTruncated: false,
          stderrTruncated: false,
          running: false,
          transcript: [
            {
              kind: 'input',
              body: '[redacted]',
              requestId: 'request-1',
              prompt: 'Password',
              secret: true,
              redacted: true,
              maxLength: 4096,
              sequence: 1,
              atEpochMs: 1,
              pending: false
            }
          ]
        }
      ]
    }
  };
}

test('saveShellSettings allows quick command transcript secret but rejects arbitrary secret-like settings', async () => {
  const calls = installInvokeRecorder();
  const validSettings = shellSettingsWithTranscriptSecret();

  const saved = await saveShellSettings(validSettings);

  assert.equal(saved, validSettings);
  assert.deepEqual(calls, [
    {
      command: 'save_shell_settings',
      args: { settings: validSettings }
    }
  ]);

  await assert.rejects(
    () =>
      saveShellSettings({
        ...defaultShellSettings(),
        ui: {
          ...defaultShellSettings().ui,
          apiSecret: 'not-allowed'
        }
      }),
    /Settings must not store secret-like key: ui\.apiSecret/
  );
  assert.equal(calls.length, 1, 'arbitrary secret-like settings must reject before invoke');
});
