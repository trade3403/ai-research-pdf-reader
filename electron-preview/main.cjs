'use strict';
const { app, BrowserWindow, ipcMain, dialog, shell, WebContentsView } = require('electron');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { isTrustedLocal, isChatOrigin } = require('./policy.cjs');
const localUrl = pathToFileURL(path.join(__dirname, 'index.html')).href;
let win = null, chat = null;
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
  win.webContents.on('will-navigate', (event, url) => { if (url !== localUrl) event.preventDefault(); });
  win.on('resize', layout);
  win.on('close', closeChat);
  win.on('closed', () => { win = null; });
  win.loadFile('index.html');
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
app.whenReady().then(createWindow);
app.on('window-all-closed', () => app.quit());
