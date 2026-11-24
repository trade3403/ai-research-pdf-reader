/** Drafts are scoped to the open PDF tab rather than the shared ChatGPT WebView2. */
export function currentPaperPrompt(drafts: Readonly<Record<string, string>>, tabId: string | null): string {
  return tabId ? drafts[tabId] ?? '' : '';
}
export function replacePaperPrompt(drafts: Readonly<Record<string, string>>, tabId: string, prompt: string): Record<string, string> {
  return { ...drafts, [tabId]: prompt };
}
export function removePaperPrompt(drafts: Readonly<Record<string, string>>, tabId: string): Record<string, string> {
  const next = { ...drafts };
  delete next[tabId];
  return next;
}
/** A deferred fill is valid only while the originating PDF remains selected. */
export function shouldFillPaperPrompt(activeId: string | null, requestTabId: string | null, requestId: number, handledId: number, prompt: string): boolean {
  return Boolean(activeId && requestTabId === activeId && requestId > handledId && prompt.trim());
}
