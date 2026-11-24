import * as pdfjs from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
import type { PDFDocumentProxy } from 'pdfjs-dist';

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export type PdfDocument = PDFDocumentProxy;
export type PdfOutlineItem = {
  title: string;
  dest: string | unknown[] | null;
  items: PdfOutlineItem[];
};

export async function openPdf(bytes: Uint8Array): Promise<PdfDocument> {
  // PDF.js takes ownership of the typed array and may transfer its buffer
  // to a worker. Do not try to reuse bytes after calling getDocument.
  const task = pdfjs.getDocument({
    data: bytes,
    useSystemFonts: true,
    cMapUrl: '/cmaps/',
    cMapPacked: true,
    standardFontDataUrl: '/standard_fonts/',
    wasmUrl: '/wasm/',
  });
  task.onPassword = (setPassword: (password: string) => void, reason: number) => {
    const password = window.prompt(reason === 2 ? 'PDF 密码错误，请重新输入：' : '此 PDF 已加密，请输入密码：');
    if (password === null) {
      task.destroy();
    } else {
      setPassword(password);
    }
  };
  return task.promise;
}

export async function pageText(pdf: PdfDocument, pageNumber: number): Promise<string> {
  const page = await pdf.getPage(pageNumber);
  const content = await page.getTextContent();
  return content.items.map((item) => ('str' in item ? item.str : '')).join(' ');
}

export async function searchPdf(pdf: PdfDocument, rawQuery: string): Promise<{ page: number; hits: number }[]> {
  const query = rawQuery.trim().toLocaleLowerCase();
  if (!query) return [];
  const matches: { page: number; hits: number }[] = [];
  // Sequential traversal limits memory use and avoids excessive worker jobs.
  for (let page = 1; page <= pdf.numPages; page++) {
    const text = (await pageText(pdf, page)).toLocaleLowerCase();
    let hits = 0;
    let offset = 0;
    while ((offset = text.indexOf(query, offset)) !== -1) {
      hits += 1;
      offset += Math.max(query.length, 1);
    }
    if (hits > 0) matches.push({ page, hits });
  }
  return matches;
}

export async function outlineTargetPage(pdf: PdfDocument, dest: string | unknown[] | null): Promise<number | null> {
  if (!dest) return null;
  try {
    const destination = typeof dest === 'string' ? await pdf.getDestination(dest) : dest;
    if (!destination || !destination.length) return null;
    const target = destination[0];
    const index = typeof target === 'number'
      ? target
      : await pdf.getPageIndex(target as Parameters<typeof pdf.getPageIndex>[0]);
    return index + 1;
  } catch {
    return null;
  }
}
