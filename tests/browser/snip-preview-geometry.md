# Snip preview browser geometry fixture

Run `npx vite --config tests/browser/snip-preview.vite.config.ts`, then open
`http://127.0.0.1:1439/tests/browser/snip-preview-geometry.html` in the visible browser panel.

Four actual iframe viewports (320×264 and 280×220, normal/wrapped-error stress)
mount the production component with production global CSS, theme/preferences,
and Tauri's mocked IPC. A real canvas PNG loads through production's load-event
fallback. The embedded panel left `decode()` pending despite `complete=true`
and `naturalWidth=126`, so the fixture deliberately disables that API rather
than fabricating decode success. This does not validate native WebView2 decode.

Each PASS requires every SVG button within its viewport, footer actions below
the image, footer/header right edges aligned, no document overflow, and actual
multiline status in stress cases. Stress text is fixture-injected after a real
mocked rejected Copy result; it is not claimed as a shipped native message.
Expand **Raw measured rectangles** for numerical evidence. No Tauri process,
native capture, clipboard publication, or Windows shell mutation occurs.

Observed: all four cases passed in the visible OpenChamber browser panel.
At 320×264, Copy rectangle is (206.4,225.6,32,32), bottom 257.6.
At 280×220, Copy rectangle is (166.4,181.6,32,32), bottom 213.6.
Wrapped status is 28.65px tall; image frame shrinks and button bounds remain
unchanged. Component journey/guard coverage lives in
`tests/components/snipping-product.test.ts`.
