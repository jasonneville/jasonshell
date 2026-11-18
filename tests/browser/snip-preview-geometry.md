# Snip preview browser geometry fixture

Run `npx vite --config tests/browser/snip-preview.vite.config.ts`, then open
`http://127.0.0.1:1439/tests/browser/snip-preview-geometry.html` in the visible browser panel.

Four actual iframe viewports (320×264 and 280×220, normal/wrapped-error stress)
mount the production component with production global CSS, theme/preferences,
and Tauri's mocked IPC. A real canvas PNG loads through production's load-event
fallback. The embedded panel left `decode()` pending despite `complete=true`
and `naturalWidth=126`, so the fixture deliberately disables that API rather
than fabricating decode success. This does not validate native WebView2 decode.

Each PASS requires all four named buttons to fit within the viewport at 32 × 32
CSS px or larger, footer actions below the image, aligned footer/header right
edges, no document overflow, and multiline status in stress cases. A separate
glyph check accepts decorative inline SVG paths or Dismiss's loaded decorative
image, which is backed by a Material SVG asset. Stress text is fixture-injected after a real
mocked rejected Copy result; it is not claimed as a shipped native message.
Expand **Raw measured rectangles** for numerical evidence. No Tauri process,
native capture, clipboard publication, or Windows shell mutation occurs.

Observed: all four cases passed in an isolated Chromium tab. In normal and
wrapped states, Dismiss measured 32 × 32 CSS px. Its right/bottom edges were
313.41/257.41 at 320×264 and 273.41/213.41 at 280×220.
Wrapped status measured 28.66 CSS px tall; image frame shrinks and button bounds remain
unchanged. Component journey/guard coverage lives in
`tests/components/snipping-product.test.ts`.
