'use strict';
const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('preview', Object.freeze({
  info: () => ipcRenderer.invoke('preview:info'),
  pickPdf: () => ipcRenderer.invoke('preview:pick-pdf'),
  toggleChat: enabled => ipcRenderer.invoke('preview:chat-toggle', enabled),
  openExternalChat: () => ipcRenderer.invoke('preview:chat-external')
}));

contextBridge.exposeInMainWorld('studio', Object.freeze({
  chatOpen: owner => ipcRenderer.invoke('studio:chat-open',owner),
  chatBounds: (owner,rect) => ipcRenderer.invoke('studio:chat-bounds',owner,rect),
  chatClose: owner => ipcRenderer.invoke('studio:chat-close',owner),
  chatExternal: () => ipcRenderer.invoke('preview:chat-external'),
  pdfStore: bytes => ipcRenderer.invoke('studio:pdf-store',bytes),
  pdfRead: token => ipcRenderer.invoke('studio:pdf-read',token),
  azureKey: (op,key) => ipcRenderer.invoke('studio:azure-key',op,key),
  azureTranslate: request => ipcRenderer.invoke('studio:azure-translate',request),
  docxSave: (name,bytes) => ipcRenderer.invoke('studio:docx-save',name,bytes),
  research: (op,value) => ipcRenderer.invoke('studio:research',op,value),
  library: (op,value) => ipcRenderer.invoke('studio:library',op,value),
  reader: (op,value) => ipcRenderer.invoke('studio:reader',op,value)
}));
