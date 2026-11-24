// User-initiated draft insertion only. Restricted to the empty ChatGPT composer.
// Do not inspect conversation messages, extract responses or click the send button.
(() => {
  try {
    if (window.location.origin !== 'https://chatgpt.com') return 'wrong_origin';
    const prompt = __PROMPT_JSON__;
    if (typeof prompt !== 'string' || !prompt.trim()) return 'unsupported';
    const selectors = [
      '#prompt-textarea[contenteditable="true"]',
      '[data-testid="prompt-textarea"][contenteditable="true"]',
      '[contenteditable="true"][role="textbox"]',
      'textarea[name="prompt-textarea"]',
    ];
    let editor = null;
    for (const selector of selectors) {
      const node = document.querySelector(selector);
      if (!node || !node.isConnected || !node.getClientRects().length) continue;
      if (node.closest('[aria-hidden="true"]')) continue;
      if (!(node.isContentEditable || node.tagName === 'TEXTAREA')) continue;
      if (node.matches(':disabled, [aria-disabled="true"]')) continue;
      editor = node;
      break;
    }
    if (!editor) return 'not_ready';
    const existing = (editor.tagName === 'TEXTAREA' ? editor.value : editor.innerText || editor.textContent || '').trim();
    if (existing === prompt.trim()) return 'already_filled';
    if (existing) return 'occupied'; // Never overwrite a user's unsent text.
    editor.focus();
    if (editor.isContentEditable) {
      // Browser editing command produces an ordinary editing/input event for ProseMirror.
      if (!document.execCommand('insertText', false, prompt)) return 'unsupported';
    } else {
      const descriptor = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value');
      if (!descriptor || typeof descriptor.set !== 'function') return 'unsupported';
      descriptor.set.call(editor, prompt);
      editor.dispatchEvent(new Event('input', { bubbles: true }));
    }
    const updated = (editor.tagName === 'TEXTAREA' ? editor.value : editor.innerText || editor.textContent || '').trim();
    return updated ? 'filled' : 'unsupported';
  } catch (_) {
    return 'unsupported';
  }
})()
