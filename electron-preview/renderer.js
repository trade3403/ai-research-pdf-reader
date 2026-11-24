'use strict';
const status = document.querySelector('#status');
const report = error => { status.textContent = String(error.message || error); };
window.preview.info().then(info => {
  document.querySelector('#runtime').textContent = 'Electron ' + info.electron + ' / Chromium ' + info.chromium;
}).catch(report);
document.querySelector('#pick').addEventListener('click', async () => {
  try {
    const file = await window.preview.pickPdf();
    status.textContent = file ? '已选择：' + file.name + '（解析尚未接入）' : '已取消选择';
  } catch (error) { report(error); }
});
let chat = false;
document.querySelector('#chat').addEventListener('click', async event => {
  const btn = event.currentTarget;
  btn.disabled = true;
  try { chat = await window.preview.toggleChat(!chat); btn.textContent = chat ? '关闭 ChatGPT' : 'ChatGPT 分屏'; }
  catch (error) { report(error); }
  finally { btn.disabled = false; }
});
document.querySelector('#external').addEventListener('click', () => window.preview.openExternalChat().catch(report));
