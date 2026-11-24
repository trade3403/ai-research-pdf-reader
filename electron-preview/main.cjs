'use strict';
const { app, BrowserWindow, ipcMain, dialog, shell, WebContentsView, protocol, net } = require('electron');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { isTrustedLocal, isChatOrigin } = require('./policy.cjs');
const localUrl = pathToFileURL(path.join(__dirname, 'index.html')).href;
let win = null, chat = null;
const workspaceUrl = 'studio://local/index.html';
protocol.registerSchemesAsPrivileged([{ scheme: 'studio', privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true, stream: true } }]);
function registerWorkspace() {
  protocol.handle('studio', request => {
    try {
      const u = new URL(request.url);
      if (u.hostname !== 'local' || request.method !== 'GET') return new Response('Forbidden', { status: 403 });
      const root = path.join(__dirname, 'workspace');
      const rel = decodeURIComponent(u.pathname).replace(/^\/+/, '') || 'index.html';
      if (rel.includes('\\') || rel.includes(':') || rel.split('/').some(x => x === '..' || x === '.')) return new Response('Forbidden', { status: 403 });
      const target = path.resolve(root, rel);
      if (!target.startsWith(root + path.sep)) return new Response('Forbidden', { status: 403 });
      return net.fetch(pathToFileURL(target).href).then(response => {
        const headers = new Headers(response.headers);
        headers.set('Content-Security-Policy', "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data: blob:; worker-src 'self' blob:; connect-src 'self' blob: https://api.crossref.org; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'");
        return new Response(response.body, { status: response.status, headers });
      }).catch(() => new Response('Not found', { status: 404 }));
    } catch { return new Response('Bad request', { status: 400 }); }
  });
}
const prefs = { contextIsolation: true, sandbox: true, nodeIntegration: false, webSecurity: true };
function authorize(event) {
  if (!win || event.sender !== win.webContents || !event.senderFrame ||
      event.senderFrame !== win.webContents.mainFrame || !isTrustedLocal(event.senderFrame.url, localUrl)) {
    throw new Error('Unauthorized IPC');
  }
}
function layout() {
  if (!win || !chat) return;
  const [w, h] = win.getContentSize();
  const left = Math.round(w * .54);
  chat.setBounds({ x: left, y: 68, width: Math.max(0, w-left), height: Math.max(0, h-68) });
}
function closeChat() {
  if (!chat) return;
  const old = chat; chat = null;
  if (win && !win.isDestroyed()) win.contentView.removeChildView(old);
  if (!old.webContents.isDestroyed()) old.webContents.close();
}
function openChat() {
  if (chat) return;
  chat = new WebContentsView({ webPreferences: { ...prefs, partition: 'persist:research-chat-preview' } });
  const wc = chat.webContents;
  wc.session.setPermissionRequestHandler((_wc, _permission, cb) => cb(false));
  wc.session.setPermissionCheckHandler(() => false);
  wc.setWindowOpenHandler(() => ({ action: 'deny' }));
  wc.on('will-navigate', (event, url) => { if (!isChatOrigin(url)) event.preventDefault(); });
  wc.on('will-redirect', (event, url) => { if (!isChatOrigin(url)) event.preventDefault(); });
  win.contentView.addChildView(chat);
  layout();
  wc.loadURL('https://chatgpt.com/').catch(() => {});
}
function createWindow() {
  win = new BrowserWindow({
    width: 1400, height: 880, minWidth: 900, minHeight: 620,
    title: 'AI Research Scientist Studio — Electron Preview',
    webPreferences: { ...prefs, preload: path.join(__dirname, 'preload.cjs') }
  });
  win.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  win.webContents.on('will-navigate', (event, url) => { if (url !== localUrl && url !== workspaceUrl) event.preventDefault(); });
  win.on('resize', layout);
  win.on('close', closeChat);
  win.on('closed', () => { win = null; });
  if (process.argv.includes('--shell-preview')) win.loadFile('index.html');
  else win.loadURL(workspaceUrl);
}
ipcMain.handle('preview:info', event => {
  authorize(event);
  return { electron: process.versions.electron, chromium: process.versions.chrome, version: app.getVersion() };
});
ipcMain.handle('preview:pick-pdf', async event => {
  authorize(event);
  const result = await dialog.showOpenDialog(win, { properties: ['openFile'], filters: [{ name: 'PDF', extensions: ['pdf'] }] });
  return result.canceled ? null : { name: path.basename(result.filePaths[0]) };
});
ipcMain.handle('preview:chat-toggle', (event, enabled) => {
  authorize(event);
  if (typeof enabled !== 'boolean') throw new Error('Invalid argument');
  if (enabled) openChat(); else closeChat();
  return Boolean(chat);
});
ipcMain.handle('preview:chat-external', event => {
  authorize(event);
  return shell.openExternal('https://chatgpt.com/');
});
app.whenReady().then(() => { registerWorkspace(); createWindow(); });
app.on('window-all-closed', () => app.quit());
