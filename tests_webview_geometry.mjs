import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {
  toPhysicalWebviewRect, matchesPhysicalWebviewRect, physicalWebviewGeometryKey,
} from './src/utils/nativeWebviewGeometry.ts';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const dock = read('./src/components/ChatBrowserDock.tsx');
const app = read('./src/App.tsx');
const css = read('./src/styles.css');
const workflow = read('./.github/workflows/build-windows-installer.yml');
const permissions = JSON.parse(read('./src-tauri/capabilities/default.json'));

for (const dpi of [1, 1.25, 1.5, 1.75, 2]) {
  const cssRect = {left: 500, top: 140, width: 390, height: 520};
  const physical = toPhysicalWebviewRect(cssRect, dpi);
  assert.deepEqual(physical, {
    x: Math.round(500 * dpi), y: Math.round(140 * dpi),
    width: Math.round(390 * dpi), height: Math.round(520 * dpi),
  });
  assert.equal(matchesPhysicalWebviewRect(physical, physical), true);
  // Reproduces the Windows 150% overlay: unscaled CSS bounds are wrong.
  if (dpi !== 1) assert.equal(matchesPhysicalWebviewRect(physical, {
    x: cssRect.left, y: cssRect.top, width: cssRect.width, height: cssRect.height,
  }), false);
}
assert.equal(matchesPhysicalWebviewRect(
  {x: 750, y: 210, width: 585, height: 780},
  {x: 752, y: 209, width: 584, height: 784},
), true);
assert.equal(matchesPhysicalWebviewRect(
  {x: 750, y: 210, width: 585, height: 780},
  {x: 520, y: 140, width: 390, height: 520},
), false);
assert.notEqual(physicalWebviewGeometryKey({x:600,y:200,width:400,height:500}),
  physicalWebviewGeometryKey({x:800,y:200,width:400,height:500}));
assert.throws(() => toPhysicalWebviewRect({left:1,top:1,width:1,height:1},0));

assert.match(dock, /getBoundingClientRect\(\)/);
assert.match(dock, /window\.devicePixelRatio/);
assert.match(dock, /toPhysicalWebviewRect\(/);
assert.match(dock, /new PhysicalPosition\(physical\.x, physical\.y\)/);
assert.match(dock, /new PhysicalSize\(physical\.width, physical\.height\)/);
assert.match(dock, /target\.position\(\), target\.size\(\)/);
assert.match(dock, /x: nativePosition\.x/);
assert.match(dock, /y: nativePosition\.y/);
assert.doesNotMatch(dock, /nativePosition\.y\s*-\s*hostPosition\.y/);
assert.doesNotMatch(dock, /host\.innerPosition\(\)/);
// Real Windows screenshot: native y=119 was incorrectly reduced by the
// 23px title-bar/client-area offset to y=96. Child coordinates are parent-relative.
const screenshotExpected = {x:0,y:119,width:1920,height:808};
const screenshotNativeReadback = {x:0,y:119,width:1920,height:808};
assert.equal(matchesPhysicalWebviewRect(screenshotExpected, screenshotNativeReadback), true);
assert.equal(matchesPhysicalWebviewRect(screenshotExpected, {...screenshotNativeReadback,y:96}), false);
assert.match(dock, /matchesPhysicalWebviewRect\(physical, actual\)/);
assert.match(dock, /await target\.hide\(\)/);
assert.match(dock, /x: 0, y: 0, width: 1, height: 1/);
assert.match(dock, /observer\.observe\(mountRef\.current\.parentElement\)/);
assert.match(dock, /window\.addEventListener\('scroll', scheduleAlign, true\)/);
assert.match(dock, /viewport\?\.addEventListener\('resize', scheduleAlign\)/);
assert.match(dock, /geometryTimer = setInterval\(scheduleAlign, 250\)/);
assert.match(dock, /if \(key === lastGeometry\) return true/);
assert.match(dock, /childRef\.current !== target/);
assert.match(dock, /clearInterval\(geometryTimer\)/);
assert.match(dock, /window\.removeEventListener\('scroll', scheduleAlign, true\)/);
assert.match(app, /chat-resize-divider/);
assert.match(app, /chat-pdf-collapsed/);
assert.match(css, /chat-native-viewport/);
assert.match(workflow, /test:webview-geometry/);
assert.deepEqual(permissions.webviews, ['main']);
for (const permission of [
  'core:webview:allow-webview-position',
  'core:webview:allow-webview-size',
]) assert.ok(permissions.permissions.includes(permission), permission);
assert.equal(dock.includes('proxyUrl'), false);
assert.equal(dock.includes('chatProxy'), false);
console.log('WebView2 physical DPI conversion, measured bounds and right-dock safety regressions passed');
