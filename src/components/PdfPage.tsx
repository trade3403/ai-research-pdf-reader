import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { TextLayer } from 'pdfjs-dist';
import type { RenderTask } from 'pdfjs-dist';
type PageViewport = ReturnType<Awaited<ReturnType<PdfDocument['getPage']>>['getViewport']>;
import type { PdfDocument } from '../utils/pdf';
import { COLOR_HEX, validRects } from '../utils/annotations';
import type { Annotation, PdfRect, SelectionDraft } from '../utils/annotations';
import { normalizePdfSelectionRects } from '../utils/pdfSelectionGeometry';

type Props = {
  pdf: PdfDocument;
  pageNumber: number;
  zoom: number | 'fit';
  rotation: number;
  query: string;
  availableWidth: number;
  annotations: Annotation[];
  selectedAnnotationId: string | null;
  onSelectText: (selection: SelectionDraft) => void;
  onSelectionContextMenu?: (event: {clientX:number;clientY:number}) => void;
  onActivateAnnotation: (id: string) => void;
};

/** Search styling must never replace PDF.js text nodes or break native Range selection. */
function markMatches(container: HTMLElement, query: string) {
  const needle = query.trim().toLocaleLowerCase();
  for (const span of container.querySelectorAll('span')) {
    span.classList.toggle('pdf-search-hit',
      Boolean(needle) && (span.textContent || '').toLocaleLowerCase().includes(needle));
  }
}

function toViewportRect(viewport: PageViewport, rect: PdfRect) {
  const a = viewport.convertToViewportPoint(rect[0], rect[1]);
  const b = viewport.convertToViewportPoint(rect[2], rect[3]);
  const left = Math.min(a[0], b[0]);
  const top = Math.min(a[1], b[1]);
  return { left, top, width: Math.abs(b[0] - a[0]), height: Math.abs(b[1] - a[1]) };
}

/** A selection rectangle is clipped to the visible page before mapping into native PDF points. */
function toPdfRect(viewport: PageViewport, sheet: DOMRect, client: {left:number;top:number;right:number;bottom:number}): PdfRect | null {
  const left = Math.max(0, client.left - sheet.left);
  const top = Math.max(0, client.top - sheet.top);
  const right = Math.min(viewport.width, client.right - sheet.left);
  const bottom = Math.min(viewport.height, client.bottom - sheet.top);
  if (right - left < 1 || bottom - top < 1) return null;
  const a = viewport.convertToPdfPoint(left, top);
  const b = viewport.convertToPdfPoint(right, bottom);
  return [a[0], a[1], b[0], b[1]];
}

export default function PdfPage({
  pdf, pageNumber, zoom, rotation, query, availableWidth,
  annotations, selectedAnnotationId, onSelectText, onSelectionContextMenu, onActivateAnnotation,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const textLayerRef = useRef<HTMLDivElement | null>(null);
  const sheetRef = useRef<HTMLDivElement | null>(null);
  const viewportRef = useRef<PageViewport | null>(null);
  const queryRef = useRef(query);
  queryRef.current = query;
  const [dimensions, setDimensions] = useState({ width: 595, height: 842, scale: 1 });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [renderVersion, setRenderVersion] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let renderTask: RenderTask | null = null;
    let textLayer: TextLayer | null = null;
    viewportRef.current = null;
    setLoading(true);
    setError('');

    (async () => {
      try {
        const page = await pdf.getPage(pageNumber);
        if (cancelled) return;
        const normal = page.getViewport({ scale: 1, rotation });
        const scale = zoom === 'fit'
          ? Math.min(3, Math.max(0.35, (Math.max(320, availableWidth) - 64) / normal.width))
          : zoom;
        const viewport = page.getViewport({ scale, rotation });
        if (cancelled) return;
        setDimensions({ width: viewport.width, height: viewport.height, scale });
        const canvas = canvasRef.current;
        const container = textLayerRef.current;
        if (!canvas || !container) return;
        container.replaceChildren();
        const context = canvas.getContext('2d', { alpha: false });
        if (!context) throw new Error('无法创建 PDF 渲染画布');
        const outputScale = Math.min(window.devicePixelRatio || 1, 2);
        canvas.width = Math.ceil(viewport.width * outputScale);
        canvas.height = Math.ceil(viewport.height * outputScale);
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;
        renderTask = page.render({ canvas, canvasContext: context, viewport,
          transform: [outputScale, 0, 0, outputScale, 0, 0] });
        await renderTask.promise;
        if (cancelled) return;
        const textContent = await page.getTextContent();
        if (cancelled) return;
        textLayer = new TextLayer({ textContentSource: textContent, container, viewport });
        await textLayer.render();
        if (cancelled) return;
        // Keep search decoration separate from PDF.js's text-span geometry.
        markMatches(container, queryRef.current);
        viewportRef.current = viewport;
        setRenderVersion((n) => n + 1);
        setLoading(false);
      } catch (e) {
        if (!cancelled) {
          setLoading(false);
          setError(e instanceof Error ? e.message : String(e));
        }
      }
    })();
    return () => { cancelled = true; viewportRef.current = null; renderTask?.cancel(); textLayer?.cancel(); };
  }, [pdf, pageNumber, zoom, rotation, availableWidth]);

  // Typing in PDF search must not destroy/recreate the text layer mid-selection.
  useEffect(() => {
    if (!loading && !error && textLayerRef.current) {
      markMatches(textLayerRef.current, query);
    }
  }, [query, loading, error, renderVersion]);

  const overlayRects = useMemo(() => {
    const viewport = viewportRef.current;
    if (!viewport || loading || error) return [];
    return annotations.flatMap((annotation) => validRects(annotation.rects).map((rect, index) => ({
      id: annotation.id, key: `${annotation.id}-${index}`, kind: annotation.kind,
      color: COLOR_HEX[annotation.color] || COLOR_HEX.yellow,
      selected: annotation.id === selectedAnnotationId,
      bounds: toViewportRect(viewport, rect),
    })));
    // renderVersion changes when PDF.js supplies a new page viewport.
  }, [annotations, selectedAnnotationId, loading, error, renderVersion]);

  const selectText = useCallback(() => {
    const selection = window.getSelection();
    const layer = textLayerRef.current;
    const sheet = sheetRef.current;
    const viewport = viewportRef.current;
    if (!selection || selection.isCollapsed || !layer || !sheet || !viewport || loading) return;
    if (selection.rangeCount !== 1) return;
    const range = selection.getRangeAt(0);
    if (!layer.contains(range.startContainer) || !layer.contains(range.endContainer)) return;
    const quote = selection.toString().trim().replace(/\s+/g, ' ');
    if (!quote || quote.length > 20000) return;
    const sheetBounds = sheet.getBoundingClientRect();
    // PDF.js can emit overlapping/duplicate DOM rects. Clip and normalize them
    // before mapping back to PDF user-space, avoiding huge highlight blocks.
    const selected = normalizePdfSelectionRects(Array.from(range.getClientRects()), sheetBounds);
    const rects = selected.map((rect) => toPdfRect(viewport, sheetBounds, rect)).filter((r): r is PdfRect => r !== null);
    if (rects.length === 0) return;
    const bounds = selected[selected.length - 1];
    onSelectText({ page: pageNumber, quote, rects, clientX: bounds.right, clientY: bounds.bottom });
  }, [loading, onSelectText, pageNumber]);

  const activateAtPoint = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    // Clicking an existing highlight opens its note; a new selection takes precedence.
    const selection = window.getSelection();
    if (selection && !selection.isCollapsed) return;
    const viewport = viewportRef.current;
    const sheet = sheetRef.current;
    if (!viewport || !sheet) return;
    const bounds = sheet.getBoundingClientRect();
    const x = event.clientX - bounds.left;
    const y = event.clientY - bounds.top;
    const annotation = [...annotations].reverse().find((ann) => ann.rects.some((r) => {
      const box = toViewportRect(viewport, r);
      return x >= box.left - 2 && x <= box.left + box.width + 2 &&
        y >= box.top - 2 && y <= box.top + box.height + 2;
    }));
    if (annotation) onActivateAnnotation(annotation.id);
  }, [annotations, onActivateAnnotation]);

  return (
    <div className="page-stage" aria-label={`PDF 第 ${pageNumber} 页`}>
      <div ref={sheetRef} className="pdf-sheet" onContextMenu={(event) => {
          const selection = window.getSelection();
          if (!selection || selection.isCollapsed || !textLayerRef.current || !selection.rangeCount) return;
          const range = selection.getRangeAt(0);
          if (!textLayerRef.current.contains(range.startContainer) || !textLayerRef.current.contains(range.endContainer)) return;
          event.preventDefault();
          selectText();
          onSelectionContextMenu?.({clientX:event.clientX,clientY:event.clientY});
        }} onMouseUp={selectText} onClick={activateAtPoint}
        style={{ width: dimensions.width, height: dimensions.height, ['--scale-factor' as string]: dimensions.scale }}>
        <canvas ref={canvasRef} className="pdf-canvas" />
        <div className="pdf-annotations-layer" aria-hidden="true">
          {overlayRects.map((rect) => <div key={rect.key}
            className={`pdf-annotation-rect ${rect.kind} ${rect.selected ? 'selected' : ''}`}
            style={{ left: rect.bounds.left, top: rect.bounds.top,
              width: rect.bounds.width, height: rect.bounds.height,
              ['--annotation-color' as string]: rect.color }} />)}
        </div>
        <div ref={textLayerRef} className={`textLayer pdf-text-layer ${loading ? 'is-rendering' : ''}`}
          style={{ width: dimensions.width, height: dimensions.height }} />
        {loading && <div className="render-overlay">正在渲染页面…</div>}
        {error && <div className="render-overlay error">页面加载失败：{error}</div>}
      </div>
    </div>
  );
}
