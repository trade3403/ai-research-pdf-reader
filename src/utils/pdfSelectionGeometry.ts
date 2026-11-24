/** Normalize DOM Range rectangles without expanding across unrelated PDF lines.
 * Each result stays in CSS pixels; callers map it with PDF.js PageViewport. */
export type PdfClientRect = {left:number;top:number;right:number;bottom:number};
export function normalizePdfSelectionRects(
  rectangles: ReadonlyArray<PdfClientRect>,
  sheet: PdfClientRect,
): PdfClientRect[] {
  const result: PdfClientRect[] = [];
  const EPS = 0.8;
  for (const r of rectangles) {
    if (![r.left,r.top,r.right,r.bottom].every(Number.isFinite)) continue;
    const left = Math.max(r.left, sheet.left);
    const right = Math.min(r.right, sheet.right);
    const top = Math.max(r.top, sheet.top);
    const bottom = Math.min(r.bottom, sheet.bottom);
    if (right - left < 1 || bottom - top < 1) continue;
    const duplicate = result.some(s =>
      Math.abs(s.left-left) < EPS && Math.abs(s.right-right) < EPS &&
      Math.abs(s.top-top) < EPS && Math.abs(s.bottom-bottom) < EPS);
    if (!duplicate) result.push({left,top,right,bottom});
  }
  result.sort((a,b) => a.top-b.top || a.left-b.left);
  // Overlapping fragments on the same baseline belong to one selection row.
  const merged: PdfClientRect[] = [];
  for (const rect of result) {
    const previous = merged[merged.length-1];
    if (previous) {
      const h1 = previous.bottom-previous.top, h2 = rect.bottom-rect.top;
      const overlap = Math.min(previous.bottom,rect.bottom)-Math.max(previous.top,rect.top);
      const sameLine = overlap >= Math.min(h1,h2)*0.7 &&
        Math.abs(h1-h2) <= Math.max(h1,h2)*0.25;
      const connected = rect.left <= previous.right + 1.5 &&
        rect.right >= previous.left - 1.5;
      if (sameLine && connected) {
        previous.left = Math.min(previous.left, rect.left);
        previous.right = Math.max(previous.right, rect.right);
        previous.top = Math.min(previous.top, rect.top);
        previous.bottom = Math.max(previous.bottom, rect.bottom);
        continue;
      }
    }
    merged.push({...rect});
  }
  return merged;
}
