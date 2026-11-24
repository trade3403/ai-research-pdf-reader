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
  reader: (op,value) => ipcRenderer.invoke('studio:reader',op,value)
}));
