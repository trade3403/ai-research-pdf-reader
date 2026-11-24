import { useEffect, useRef, useState } from 'react';
import type { RenderTask } from 'pdfjs-dist';
import type { PdfDocument } from '../utils/pdf';

type Props = {
  pdf: PdfDocument;
  page: number;
  selected: boolean;
  onClick: () => void;
};

export default function Thumbnail({ pdf, page, selected, onClick }: Props) {
  const wrapper = useRef<HTMLButtonElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const target = wrapper.current;
    if (!target) return;
    const observer = new IntersectionObserver(
      ([entry]) => { if (entry.isIntersecting) setVisible(true); },
      { rootMargin: '160px' },
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [pdf]);

  useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    let job: RenderTask | null = null;
    (async () => {
      try {
        const p = await pdf.getPage(page);
        if (cancelled || !canvas.current) return;
        const initial = p.getViewport({ scale: 1 });
        const viewport = p.getViewport({ scale: 124 / initial.width });
        const c = canvas.current;
        const context = c.getContext('2d');
        if (!context) return;
        c.width = Math.ceil(viewport.width);
        c.height = Math.ceil(viewport.height);
        job = p.render({ canvas: c, canvasContext: context, viewport });
        await job.promise;
      } catch { /* cancelled thumbnail renders are expected */ }
    })();
    return () => {
      cancelled = true;
      job?.cancel();
    };
  }, [pdf, page, visible]);

  return (
    <button ref={wrapper} className={`thumb-item ${selected ? 'selected' : ''}`} onClick={onClick} title={`第 ${page} 页`}>
      <span className="thumb-image"><canvas ref={canvas} /></span>
      <span className="thumb-number">{page}</span>
    </button>
  );
}
