const test = require('node:test');
const assert = require('node:assert/strict');
const { isTrustedLocal, isChatOrigin } = require('../policy.cjs');
test('exact local URL only', () => {
  assert.equal(isTrustedLocal('file:///app/index.html','file:///app/index.html'), true);
  assert.equal(isTrustedLocal('file:///app/index.html?x','file:///app/index.html'), false);
  assert.equal(isTrustedLocal('https://chatgpt.com/','file:///app/index.html'), false);
});
test('restrict remote navigation', () => {
  assert.equal(isChatOrigin('https://chatgpt.com/c/123'), true);
  for (const u of ['http://chatgpt.com','https://chatgpt.com.evil.example','file:///secret','https://evil.example','https://user@chatgpt.com','https://chatgpt.com:444']) assert.equal(isChatOrigin(u),false);
});
