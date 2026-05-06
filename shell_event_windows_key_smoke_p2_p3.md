# Shell Event And Windows Key Smoke P2/P3

Status: Automated validation complete; live manual smoke not run by agent.

Date: 2026-05-06

## Scope

- `shell_open_close_event_correctness_p2.md`
- `windows_key_chord_preservation_p3.md`

## Automated coverage completed

- Search native and explicit close now target `top-bar` through `emit_search_panel_closed_to_top_bar`.
- Audio close now targets both `top-bar` and `audio-panel`; `AudioPanelSurface.svelte` stops polling on its own close event.
- Tray show now emits `tray-panel:open` to `tray-panel`; `TrayPanelSurface.svelte` reloads icons on every open event.
- Windows-key classifier now passes Windows-key down and chord release paths through to Windows, suppressing only bare final key release to open centered search and avoid Start activation.

## Manual smoke not run

The real Windows shell smoke was not run by the agent because it requires interactive desktop control and includes `Win+L`, which locks the active Windows session. Running it without the user present would disrupt the session.

## Manual smoke checklist

1. Start JasonShell normally.
2. Open centered search, click desktop/outside, press bare Windows key again. Expected: centered search opens blank/fresh; Start does not flash or focus.
3. Open sound panel, click away, wait more than 2 seconds. Expected: no further `get_audio_state` polling while hidden.
4. Open tray panel twice after changing or mocking tray icons. Expected: second open shows a fresh icon snapshot.
5. Press `Win+R`. Expected: Windows Run opens.
6. Press `Win+D`. Expected: desktop toggles.
7. Press `Win+E`. Expected: File Explorer opens.
8. Press `Win+L` only with user consent. Expected: Windows locks the session.
9. Exit JasonShell. Expected: normal Windows-key behavior returns.

