'use strict';
function isTrustedLocal(senderUrl, expectedUrl) {
  return typeof senderUrl === 'string' && senderUrl === expectedUrl;
}
function isChatOrigin(raw) {
  try {
    const u = new URL(raw);
    return u.protocol === 'https:' && u.origin === 'https://chatgpt.com' && !u.username && !u.password;
  } catch { return false; }
}
module.exports = { isTrustedLocal, isChatOrigin };
