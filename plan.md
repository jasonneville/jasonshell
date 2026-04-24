# Bottom-bar task windows

1. Keep the shell split intact: the bottom bar owns pinned launchers on the left and open-window tabs to the right, with icon-only launchers and labeled task buttons for live windows.
2. Add two backend window commands for the bottom bar: `list_open_task_windows` returns stable HWND-backed window rows, and `activate_task_window` toggles inactive, minimized, and active windows with taskbar-like behavior.
3. Preserve Explorer-compatible launcher behavior: read pinned shortcuts from the taskbar pin directory, execute the `.lnk` itself when launching, and improve icon fidelity by resolving explicit shortcut icons or target icons before falling back to the shortcut file.
4. Keep the v1 window model simple: one tab per eligible top-level window on the primary monitor, no grouping, previews, or multi-monitor expansion.
5. Validation checkpoints:
   - static: `npm run check`, `npm run cargo:test`, `npm run cargo:check`
   - backend shape: pinned launchers still enumerate, `.lnk` launch still works, open-window commands return stable IDs, titles, and icons
   - runtime: bottom bar shows icon-only launchers, window tabs activate or minimize the correct HWND, and the strip tolerates windows appearing, disappearing, and minimizing
6. Key constraints:
   - do not reintroduce top-bar remediation work into this plan
   - do not break `.lnk` launch semantics while improving launcher icon fidelity
   - preserve Explorer taskbar hiding and AppBar ownership while extending the bottom bar
