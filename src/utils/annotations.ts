/**
 * All rectangles are stored in native PDF user-space points, not device pixels.
 * This makes annotations independent of zoom and the viewer's current rotation.
 */
export type PdfRect = [number, number, number, number];
export type AnnotationKind = 'highlight' | 'underline' | 'strike';
export type AnnotationColor = 'yellow' | 'green' | 'blue' | 'pink';

export type SelectionDraft = {
  page: number;
  quote: string;
  rects: PdfRect[];
  clientX: number;
  clientY: number;
};

export type Annotation = {
  id: string;
  fingerprint: string;
  page: number;
  kind: AnnotationKind;
  color: AnnotationColor;
  quote: string;
  note: string;
  rects: PdfRect[];
  createdAt: string;
  updatedAt: string;
};

export const COLOR_HEX: Record<AnnotationColor, string> = {
  yellow: '#f9cb50',
  green: '#76ce99',
  blue: '#83baf9',
  pink: '#f6a5be',
};

/** Ensure malformed imported / stored geometry never produces unsafe overlay CSS. */
export function validRects(rects: PdfRect[]): PdfRect[] {
  return rects.filter((r) => r.length === 4 && r.every((n) => Number.isFinite(n)));
}

export function markdownForAnnotations(name: string, annotations: Annotation[]): string {
  const clean = (value: string) => value.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const lines = [`# ${clean(name).replace(/\n/g, ' ')}`, '', 'PDF 阅读批注导出', '', `导出时间：${new Date().toLocaleString()}`, ''];
  const ordered = [...annotations].sort((a, b) => a.page - b.page || a.createdAt.localeCompare(b.createdAt));
  for (const ann of ordered) {
    lines.push(`## 第 ${ann.page} 页 · ${ann.kind === 'highlight' ? '高亮' : ann.kind === 'underline' ? '下划线' : '删除线'}`, '');
    for (const line of clean(ann.quote).split('\n')) lines.push(`> ${line}`);
    lines.push('');
    if (ann.note.trim()) lines.push('**笔记**', '', clean(ann.note), '');
    lines.push(`颜色：${ann.color} · 保存时间：${ann.updatedAt}`, '', '---', '');
  }
  if (!ordered.length) lines.push('暂无批注。', '');
  return lines.join('\n');
}
