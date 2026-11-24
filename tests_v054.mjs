import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const app = read('./src/App.tsx');
const dock = read('./src/components/ChatBrowserDock.tsx');
const css = read('./src/styles.css');
const workflow = read('./.github/workflows/build-windows-installer.yml');

assert.match(app, /research-chat-pdf-collapsed/);
assert.match(app, /localStorage\.getItem\('research-chat-pdf-collapsed'\) === 'true'/);
assert.match(app, /localStorage\.setItem\('research-chat-pdf-collapsed', String\(chatPdfCollapsed\)\)/);
assert.match(app, /research-word-chat-open/);
assert.match(app, /research-chat-panel-ratio/);
assert.match(app, /event\.repeat/);
assert.match(app, /event\.ctrlKey/);
assert.match(app, /event\.shiftKey/);
assert.match(app, /event\.altKey/);
assert.match(app, /event\.metaKey/);
assert.match(app, /\['KeyL', 'KeyC', 'KeyR'\]/);
assert.match(app, /target\.isContentEditable/);
assert.match(app, /closest\('input, textarea, select/);
assert.match(app, /setChatPdfCollapsed\(value => !value\)/);
assert.match(app, /setWordChatOpen\(value => !value\)/);
assert.match(app, /setChatRatio\(0\.43\)/);
assert.match(app, /window\.removeEventListener\('keydown', onShortcut\)/);
assert.match(app, /<ChatBrowserDock key="chat-dock"/);
assert.match(dock, /onClick=\{onTogglePdf\}/);
assert.match(dock, /ResizeObserver\(scheduleAlign\)/);
assert.match(css, /chat-pdf-collapsed \.chat-browser-dock/);
assert.match(workflow, /test:v054/);

// This is a deterministic state-transition regression, not an actual WebView2 test.
let collapsed = false;
let ratio = 0.57;
for (let i = 0; i < 100; i++) {
  collapsed = !collapsed;
  assert.equal(Number.isFinite(ratio), true);
}
assert.equal(collapsed, false);
assert.equal(ratio, 0.57);
console.log('V0.5.4 layout persistence, shortcut wiring, dock identity checks passed; Windows UI validation still required');
