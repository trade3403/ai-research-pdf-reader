// Static smoke guard for V0.2 source organization. This does not replace Windows UI tests.
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const load = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const rust = load('./src-tauri/src/lib.rs');
const store = load('./src/utils/readerStore.ts');
const page = load('./src/components/PdfPage.tsx');
const app = load('./src/App.tsx');
assert.match(rust, /read_recent_pdf/);
assert.match(rust, /write_markdown/);
assert.match(store, /CREATE TABLE IF NOT EXISTS annotations/);
assert.match(store, /ON CONFLICT\(id\)/);
assert.match(page, /convertToPdfPoint/);
assert.match(page, /convertToViewportPoint/);
assert.match(app, /markdownForAnnotations/);
assert.match(app, /setPendingSelection/);
console.log('V0.2 structural checks passed');
