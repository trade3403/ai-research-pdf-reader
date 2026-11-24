'use strict';
const { app, BrowserWindow, ipcMain, dialog, shell, WebContentsView, protocol, net, safeStorage } = require('electron');
const path = require('node:path');
const fs = require('node:fs/promises');
const {randomUUID}=require('node:crypto');
const {createBackup,verifyBackup}=require('./backup.cjs');
const {createIncremental,materializeIncremental}=require('./incremental.cjs');
const {inspectLegacy}=require('./legacy-preflight.cjs');
const {stageRestore,stageRollback,activatePending,state}=require('./restore.cjs');
let dataRoot=null;
const primaryInstance=app.requestSingleInstanceLock();
if(!primaryInstance)app.quit();
const {createKeyStore,translateBatch}=require('./azure-translator.cjs');
const { pathToFileURL } = require('node:url');
const { isTrustedLocal, isChatOrigin } = require('./policy.cjs');
const localUrl = pathToFileURL(path.join(__dirname, 'index.html')).href;
let win = null, chat = null, chatOwner = null, readerDb = null;
const {createReaderDb} = require('./reader-db.cjs');
const {createVault}=require('./pdf-vault.cjs');
const {createLibraryDb}=require('./library-db.cjs');
let vault=null,libraryDb=null,researchDb=null;
const {createResearchDb}=require('./research-db.cjs');
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
      event.senderFrame !== win.webContents.mainFrame || !(isTrustedLocal(event.senderFrame.url, localUrl) || isTrustedLocal(event.senderFrame.url, workspaceUrl))) {
    throw new Error('Unauthorized IPC');
  }
}
function layout() {
  if (!win || !chat || chatOwner) return;
  const [w, h] = win.getContentSize();
  const left = Math.round(w * .54);
  chat.setBounds({ x: left, y: 68, width: Math.max(0, w-left), height: Math.max(0, h-68) });
}
function closeChat() {
  if (!chat) return;
  const old = chat; chat = null; chatOwner = null;
  if (win && !win.isDestroyed()) win.contentView.removeChildView(old);
  if (!old.webContents.isDestroyed()) old.webContents.close();
}
function openChat(owner = null) {
  if (chat) return;
  chat = new WebContentsView({ webPreferences: { ...prefs, partition: 'persist:research-chat-preview' } });
  chatOwner = owner;
  if (owner) chat.setVisible(false);
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
function checkOwner(owner) { if(typeof owner!=='string'||!/^[a-f0-9-]{36}$/.test(owner)) throw new Error('Invalid dock owner'); }
ipcMain.handle('studio:chat-open',(event,owner)=>{
  authorize(event);checkOwner(owner);
  if(chat && chatOwner!==owner) closeChat();
  openChat(owner);
  return true;
});
ipcMain.handle('studio:chat-bounds',(event,owner,rect)=>{
  authorize(event);checkOwner(owner);
  if(!chat||chatOwner!==owner) return false;
  if(rect===null){chat.setVisible(false);return true;}
  if(!rect||!['x','y','width','height'].every(k=>Number.isFinite(rect[k]))) throw new Error('Invalid dock bounds');
  const [w,h]=win.getContentSize();
  const x=Math.max(0,Math.round(rect.x)),y=Math.max(0,Math.round(rect.y));
  const width=Math.min(Math.round(rect.width),w-x),height=Math.min(Math.round(rect.height),h-y);
  if(width<40||height<40){chat.setVisible(false);return false;}
  chat.setBounds({x,y,width,height});chat.setVisible(true);return true;
});
ipcMain.handle('studio:chat-close',(event,owner)=>{
  authorize(event);checkOwner(owner);
  if(chatOwner===owner) closeChat();
});
ipcMain.handle('studio:reader',(event,op,value)=>{
  authorize(event);
  if(event.senderFrame.url!==workspaceUrl) throw new Error('Workspace only');
  if(!readerDb) readerDb=createReaderDb(path.join(dataRoot,'reader-v062.sqlite'));
  return readerDb.call(op,value);
});
function workspaceOnly(event){authorize(event);if(event.senderFrame.url!==workspaceUrl)throw Error('Workspace only');}
ipcMain.handle('studio:pdf-store',async(event,bytes)=>{workspaceOnly(event);vault??=createVault(path.join(dataRoot,'pdf-vault'));return vault.store(bytes);});
ipcMain.handle('studio:pdf-read',async(event,token)=>{workspaceOnly(event);vault??=createVault(path.join(dataRoot,'pdf-vault'));return vault.read(token);});
ipcMain.handle('studio:library',(event,op,value)=>{workspaceOnly(event);libraryDb??=createLibraryDb(path.join(dataRoot,'library-v062.sqlite'));return libraryDb.call(op,value);});
function getResearch(){return researchDb??=createResearchDb(path.join(dataRoot,'research-v063.sqlite'));}
ipcMain.handle('studio:research',(event,op,value)=>{workspaceOnly(event);if(op==='translation.reserve')throw Error('Quota reservation is main-process only');return getResearch().call(op,value);});
function keys(){return createKeyStore(path.join(app.getPath('userData'),'secrets','azure.dpapi'),safeStorage);}
ipcMain.handle('studio:azure-key',(event,op,key)=>{workspaceOnly(event);const k=keys();switch(op){case 'save':return k.save(key);case 'has':return k.has();case 'remove':return k.remove();default:throw Error('Unsupported key operation');}});
let translating=false;
ipcMain.handle('studio:azure-translate',async(event,request)=>{
 workspaceOnly(event);if(translating)throw Error('Translation already in progress');translating=true;
 try{return await translateBatch(request,{keyStore:keys(),reserve:chars=>{
  const db=getResearch();const limit=db.call('translation.settings')?.monthlyLimit??1500000;
  db.call('translation.reserve',{chars,limit,month:new Date().toISOString().slice(0,7)});
 }});}finally{translating=false;}
});
ipcMain.handle('studio:docx-save',async(event,name,bytes)=>{
 workspaceOnly(event);
 if(typeof name!=='string'||name.length>100||!name.toLowerCase().endsWith('.docx')||/[\\/:*?"<>|]/.test(name)||!(bytes instanceof Uint8Array)||bytes.length<4||bytes.length>32*1024*1024||bytes[0]!==80||bytes[1]!==75)throw Error('Invalid DOCX export');
 const result=await dialog.showSaveDialog(win,{defaultPath:name,filters:[{name:'Word DOCX',extensions:['docx']}]});
 if(result.canceled||!result.filePath)return null;
 const target=result.filePath;if(!target.toLowerCase().endsWith('.docx'))throw Error('Choose a .docx file');
 const tmp=path.join(path.dirname(target),'.studio-'+randomUUID()+'.tmp');
 try{await fs.writeFile(tmp,bytes,{flag:'wx'});await fs.rename(tmp,target);return target;}finally{await fs.rm(tmp,{force:true});}
});
let backupBusy=false;
ipcMain.handle('studio:backup',(event,op)=>{
 workspaceOnly(event);
 if(!['create','verify','restore','rollback','incremental','materialize'].includes(op))throw Error('Unsupported backup operation');
 if(backupBusy||translating)throw Error('Finish current backup or translation first');
 backupBusy=true;
 return (async()=>{
  try{
   const appProfile=app.getPath('userData');
   if(op==='rollback')return await stageRollback(appProfile);
   const selected=await dialog.showOpenDialog(win,{title:['create','incremental'].includes(op)?'选择备份保存位置':op==='materialize'?'选择增量快照文件夹':'选择需要校验的备份文件夹',properties:['openDirectory']});
   if(selected.canceled||!selected.filePaths[0])return null;
   if(op==='verify')return await verifyBackup(selected.filePaths[0]);
   if(op==='materialize'){
    const dest=await dialog.showOpenDialog(win,{title:'选择完整备份导出位置',properties:['openDirectory']});if(dest.canceled||!dest.filePaths[0])return null;
    const rel=path.relative(await fs.realpath(appProfile),await fs.realpath(dest.filePaths[0]));if(rel===''||(!path.isAbsolute(rel)&&rel!=='..'&&!rel.startsWith('..'+path.sep)))throw Error('Choose export outside application profile');
    return await materializeIncremental(selected.filePaths[0],dest.filePaths[0]);
   }
   const profile=dataRoot;
   if(op==='restore'){
    readerDb??=createReaderDb(path.join(profile,'reader-v062.sqlite'));
    libraryDb??=createLibraryDb(path.join(profile,'library-v062.sqlite'));getResearch();
    return await stageRestore(appProfile,selected.filePaths[0]);
   }
   const relative=path.relative(await fs.realpath(appProfile),await fs.realpath(selected.filePaths[0]));
   if(relative===''||(!path.isAbsolute(relative)&&relative!=='..'&&!relative.startsWith('..'+path.sep)))throw Error('Choose backup outside application profile');
   readerDb??=createReaderDb(path.join(profile,'reader-v062.sqlite'));
   libraryDb??=createLibraryDb(path.join(profile,'library-v062.sqlite'));
   const options={profile,parent:selected.filePaths[0],stores:[readerDb,libraryDb,getResearch()]};
   return op==='incremental'?await createIncremental(options):await createBackup(options);
  }finally{backupBusy=false;}
 })();
});
ipcMain.handle('studio:legacy-preflight',async event=>{workspaceOnly(event);const result=await dialog.showOpenDialog(win,{title:'选择 V0.5.9 SQLite 备份',properties:['openFile'],filters:[{name:'SQLite',extensions:['sqlite','db']}]});if(result.canceled||!result.filePaths[0])return null;return inspectLegacy(result.filePaths[0]);});
app.on('will-quit',()=>{if(researchDb){researchDb.close();researchDb=null;}if(readerDb){readerDb.close();readerDb=null;}if(libraryDb){libraryDb.close();libraryDb=null;}});
ipcMain.handle('studio:restore-status',async event=>{workspaceOnly(event);const p=await state(app.getPath('userData'));return {canRollback:p.previous!==null};});
if(primaryInstance)app.whenReady().then(async()=>{
 const profile=app.getPath('userData');await fs.mkdir(profile,{recursive:true});
 const activated=await activatePending(profile);dataRoot=activated.root;
 registerWorkspace();createWindow();
 if(activated.error)dialog.showErrorBox('恢复失败，原数据已保留',activated.error);
}).catch(error=>{dialog.showErrorBox('科研数据库启动失败',String(error.message||error));app.quit();});
app.on('window-all-closed', () => app.quit());
