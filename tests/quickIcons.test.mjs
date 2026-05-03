import assert from 'node:assert/strict';
import test from 'node:test';
import {
  defaultQuickIconsSettings,
  filterExplorerLaunchersForQuickIcons,
  normalizeQuickIconTargetKey
} from '../dist-tests/lib/quickIcons.js';

test('quick icon settings default to empty entries', () => {
  assert.deepEqual(defaultQuickIconsSettings(), { entries: [] });
});

test('normalizes Windows-like quick icon target keys for duplicate detection', () => {
  assert.equal(
    normalizeQuickIconTargetKey('C:/Program Files/Google/Chrome/Application/chrome.exe'),
    'c:\\program files\\google\\chrome\\application\\chrome.exe'
  );
  assert.equal(
    normalizeQuickIconTargetKey('  \\\\Server\\Share\\Tool.exe  '),
    '\\\\server\\share\\tool.exe'
  );
});

test('keeps Explorer launchers even when legacy quick icons duplicate targets', () => {
  const quickIcons = [
    {
      id: 'chrome',
      name: 'Chrome',
      targetPath: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
      iconDataUrl: 'data:image/png;base64,aaa'
    }
  ];
  const launchers = [
    {
      id: 'launcher-chrome',
      name: 'Chrome',
      shortcutPath: 'C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar\\Chrome.lnk',
      targetPath: 'c:/program files/google/chrome/application/chrome.exe',
      iconDataUrl: 'data:image/png;base64,aaa'
    },
    {
      id: 'launcher-code',
      name: 'Code',
      shortcutPath: 'C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar\\Code.lnk',
      targetPath: 'C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe',
      iconDataUrl: 'data:image/png;base64,bbb'
    }
  ];

  const filtered = filterExplorerLaunchersForQuickIcons(quickIcons, launchers);
  assert.deepEqual(filtered.map((launcher) => launcher.name), ['Chrome', 'Code']);
});
