# External Stack refresh browser acceptance

Start `node node_modules/vite/bin/vite.js --config tests/browser/stack-external-refresh/vite.config.ts`, open http://127.0.0.1:4189 with agent-browser, click **Run external change journey**, inspect `#result` and frozen `window.stackExternalRefreshEvidence`.

The production popup/CSS are mounted with controlled watch IPC, opaque tokens, delayed summaries and an exact invocation ledger. Assertions cover automatic rows/badges, burst debounce, single-flight dirty followup, focus/query/selection retention, unchanged same-repo zero scans and subscription reuse, visible-unfocused updates, hide/reopen catchup, stale-token rejection and unmount cleanup. Actual native filesystem delivery is separately verified by `tests/native/stack_folder_watch.rs`; mocked browser IPC is not live Tauri or Windows shell certification.

**Run review P2 regressions** independently runs both delayed external-context/same-repository navigation and missing trusted Git paths. Inspect `#review-result` and `window.stackExternalRefreshReviewEvidence`; each case freezes its own checks/error/ledger so one failure does not prevent the other case running. The original journey remains separate.
