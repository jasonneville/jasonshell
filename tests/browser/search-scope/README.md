# Search scope browser acceptance

Production `SearchPanelSurface.svelte` and global CSS, mounted with rejecting mocked IPC. No native shell calls or dependencies added.

```powershell
npx vite --config tests/browser/search-scope/vite.config.ts
agent-browser --session search-scope open http://127.0.0.1:4186
agent-browser --session search-scope wait --fn "window.searchScopeReady === true"
agent-browser --session search-scope eval "window.searchScopeFixture.run()"
```

Expected `failures: []`. Covers all scopes, unchanged query emission, original activation/selection identities, hidden-result safety, select focus on refresh, type/clear and 720/420/320px geometry with long result titles. For trusted input smoke use snapshot/select/fill/press on this same mounted fixture. Browser evidence does not certify WebView2/native actions or live shell placement.

If 4186 is occupied, use `npx vite --config tests/browser/search-scope/vite.config.ts --port 4196` and open that port instead; do not stop unrelated servers.

Trusted native-select keyboard smoke: focus `select`, press `Home`, `ArrowDown`, `Tab`, then `Enter`. From All this chooses Apps and query Enter emits the original first app identity (`resultIndex: 1`, `recordKey: app:first`). Capture document key/input/change listeners' `isTrusted` values to distinguish CDP input from the synthetic events used by `run()`. The automated journey also verifies visible folder Ctrl+Enter and rejects hidden-folder pinning in Apps.
