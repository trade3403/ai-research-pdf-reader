import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const pkg = JSON.parse(read('./package.json'));
assert.ok(['0.5.3','0.5.5','0.5.6','0.5.7','0.5.8','0.5.9'].includes(pkg.version));
const app = read('./src/App.tsx');
const dock = read('./src/components/ChatBrowserDock.tsx');
const css = read('./src/styles.css');
const rust = read('./src-tauri/src/chatgpt_fill.rs');
const js = read('./src-tauri/src/chatgpt_prompt_fill.js');
const caps = JSON.parse(read('./src-tauri/capabilities/default.json'));
assert.match(app, /chatPdfCollapsed/);
assert.match(app, /setChatPdfCollapsed\(v => !v\)/);
assert.match(app, /!chatPdfCollapsed && chatResizeHandle/);
assert.match(app, /mode === 'chat' && chatPdfCollapsed/);
assert.match(app, /onPromptHandled=\{id => setChatPromptHandledId/);
assert.match(dock, /onTogglePdf/);
assert.match(dock, /显示 PDF/);
assert.match(dock, /隐藏 PDF/);
assert.match(dock, /onClick=\{onTogglePdf\}/);
assert.match(dock, /chatgpt_fill_prompt/);
assert.match(dock, /重试自动填写/);
assert.match(dock, /复制提示词/);
assert.match(css, /chat-pdf-collapsed \.chat-browser-dock/);
assert.match(rust, /get_webview\(&label\)/);
assert.match(rust, /eval_with_callback/);
assert.match(rust, /url\.host_str\(\) != Some\("chatgpt.com"\)/);
assert.match(read('./src-tauri/src/lib.rs'), /chatgpt_fill::chatgpt_fill_prompt/);
assert.deepEqual(caps.webviews, ['main']);
assert.equal(caps.remote, undefined);
assert.doesNotMatch(js, /\.click\(|\.submit\(|fetch\(|document\.cookie|querySelectorAll\(.*message|localStorage|sessionStorage/);
assert.doesNotMatch(rust, /get_webview\("main"\).*\.eval/);
assert.match(read('./.github/workflows/build-windows-installer.yml'), /test:v053/);

function evaluate(prompt, { origin = 'https://chatgpt.com', initial = '', visible = true, exists = true, contenteditable = true } = {}) {
  const editor = {
    isConnected: true,
    isContentEditable: contenteditable,
    tagName: contenteditable ? 'DIV' : 'TEXTAREA',
    innerText: initial,
    textContent: initial,
    value: initial,
    getClientRects: () => visible ? [{}] : [],
    closest: () => null,
    matches: () => false,
    focus: () => {},
    dispatchEvent: () => {},
  };
  const context = {
    window: { location: { origin } },
    document: {
      querySelector: () => exists ? editor : null,
      execCommand: (command, _flag, value) => {
        assert.equal(command, 'insertText');
        editor.innerText = value;
        editor.textContent = value;
        return true;
      },
    },
    Object,
    Event: class { constructor() {} },
    HTMLTextAreaElement: class {},
  };
  const filled = vm.runInNewContext(js.replace('__PROMPT_JSON__', JSON.stringify(prompt)), context, { timeout: 2000 });
  return { filled, editor };
}
const message = '请解释离散能量稳定性 E^{n+1} <= E^n，并给出验证方案';
assert.equal(evaluate(message).filled, 'filled');
assert.equal(evaluate(message).editor.innerText, message);
const occupied = evaluate(message, {initial:'我正在编辑的草稿'});
assert.equal(occupied.filled, 'occupied');
assert.equal(occupied.editor.innerText, '我正在编辑的草稿');
assert.equal(evaluate(message, {initial:message}).filled, 'already_filled');
assert.equal(evaluate(message, {exists:false}).filled, 'not_ready');
assert.equal(evaluate(message, {visible:false}).filled, 'not_ready');
assert.equal(evaluate(message, {origin:'https://example.com'}).filled, 'wrong_origin');
const adversarial = 'Hello `}); alert("bad") // \n *中文*';
assert.equal(evaluate(adversarial).editor.innerText, adversarial, 'Prompt is a JS string literal, not executable source');
console.log('V0.5.3 PDF collapse, user-confirmed prompt insertion, no-send and origin isolation tests passed');
