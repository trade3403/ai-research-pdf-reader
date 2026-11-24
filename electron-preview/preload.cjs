'use strict';
const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('preview', Object.freeze({
  info: () => ipcRenderer.invoke('preview:info'),
  pickPdf: () => ipcRenderer.invoke('preview:pick-pdf'),
  toggleChat: enabled => ipcRenderer.invoke('preview:chat-toggle', enabled),
  openExternalChat: () => ipcRenderer.invoke('preview:chat-external')
}));
