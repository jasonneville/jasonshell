import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  hasFolderDragPayload,
  folderPathsFromTransfer,
  folderPathToUri,
  normalizeDroppedPath,
  setFolderDragPayload
} from '../dist-tests/folderDrag.js';

test('normalizes local file uri paths', () => {
  assert.equal(
    normalizeDroppedPath('file:///C:/Dev/My%20Repo'),
    'C:\\Dev\\My Repo'
  );
  assert.equal(
    normalizeDroppedPath('file://localhost/C:/Dev/Repo'),
    'C:\\Dev\\Repo'
  );
});

test('normalizes unc file uri paths', () => {
  assert.equal(
    normalizeDroppedPath('file://server/share/My%20Repo'),
    '\\\\server\\share\\My Repo'
  );
});

test('encodes windows paths as file uris', () => {
  assert.equal(folderPathToUri('C:\\Dev\\My Repo'), 'file:///C:/Dev/My%20Repo');
  assert.equal(folderPathToUri('\\\\server\\share\\My Repo'), 'file://server/share/My%20Repo');
});

test('extracts every path from multi-line uri lists', () => {
  const transfer = {
    types: ['text/uri-list'],
    files: [],
    getData(type) {
      return type === 'text/uri-list'
        ? '# ignored\r\nfile:///C:/One\r\nfile://localhost/C:/Two%20Words\r\n'
        : '';
    }
  };

  assert.deepEqual(folderPathsFromTransfer(transfer), [
    'C:\\One',
    'C:\\Two Words'
  ]);
});

test('publishes shared folder drag payloads for top-bar drops', () => {
  const payloads = new Map();
  const transfer = {
    effectAllowed: 'uninitialized',
    setData(type, value) {
      payloads.set(type, value);
    }
  };

  setFolderDragPayload(transfer, ['C:\\One', 'C:\\Two Words'], 'copyMove');

  assert.equal(transfer.effectAllowed, 'copyMove');
  assert.equal(payloads.get('application/x-jasonshell-folder'), 'C:\\One');
  assert.equal(payloads.get('text/plain'), 'C:\\One\nC:\\Two Words');
  assert.equal(payloads.get('text/uri-list'), 'file:///C:/One\r\nfile:///C:/Two%20Words');
});

test('treats native Explorer Files drags as folder payload candidates', () => {
  assert.equal(hasFolderDragPayload({ types: ['Files'], files: [], getData() { return ''; } }), true);
});
