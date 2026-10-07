import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
const root = resolve('.');
const allowed = new Set(['/src/app.css', '/src/components/BottomBar.css', '/tests/browser/bottom-bar-insets/index.html', '/tests/browser/bottom-bar-insets/checks.js']);
http.createServer(async (req, res) => {
  const pathname = new URL(req.url, 'http://localhost').pathname;
  const path = resolve(root, `.${pathname}`);
  if (!allowed.has(pathname) || !path.startsWith(root + sep)) { res.writeHead(404).end(); return; }
  try { res.setHeader('Content-Type', extname(path) === '.css' ? 'text/css' : extname(path) === '.js' ? 'text/javascript' : 'text/html'); res.end(await readFile(path)); }
  catch { res.writeHead(404).end(); }
}).listen(5191, '127.0.0.1', () => console.log('Safe CSS fixture: http://127.0.0.1:5191/tests/browser/bottom-bar-insets/index.html'));
