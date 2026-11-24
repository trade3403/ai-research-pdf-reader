import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties, PointerEvent as UIPointerEvent } from 'react';
import {
  BookMarked, BookOpen, ChevronDown, ChevronLeft, ChevronRight, CircleAlert,
  Clock3, FilePlus2, FileSearch2, FileText, FolderOpen, Hash, List,
  Minus, Moon, PanelLeftClose, PanelLeftOpen, Plus, RotateCw, Search,
  SidebarOpen, Sun, X, ZoomIn, Highlighter, MessageSquareText, LibraryBig, Globe, NotebookPen, Languages, Sparkles, Sigma, Lightbulb, DatabaseBackup, DatabaseZap, FilePenLine,
} from 'lucide-react';
import { isTauri } from '@tauri-apps/api/core';
import PdfPage from './components/PdfPage';
import Thumbnail from './components/Thumbnail';
import AnnotationPanel from './components/AnnotationPanel';
import LiteratureLibrary from './components/LiteratureLibrary';
import ChatBrowserDock from './components/ChatBrowserDock';
import ResearchRecords from './components/ResearchRecords';
import TranslatorWorkspace from './components/TranslatorWorkspace';
import WordEditor from './components/WordEditor';
import type { WordInsert } from './components/WordEditor';
import type { ClipDraftSeed } from './components/ResearchRecords';
import { COLOR_HEX, markdownForAnnotations } from './utils/annotations';
import type { Annotation, AnnotationKind, AnnotationColor, SelectionDraft } from './utils/annotations';
import { getAnnotations, saveAnnotation, deleteAnnotation, getDocument, listDocuments, saveDocument } from './utils/readerStore';
import { getBookmarks, toggleBookmark } from './utils/bookmarks';
import { openPdf, outlineTargetPage, searchPdf } from './utils/pdf';
import type { PdfDocument, PdfOutlineItem } from './utils/pdf';
import { buildResearchPrompt, type ResearchPromptKind } from './utils/researchPrompts';
import { currentPaperPrompt, removePaperPrompt, replacePaperPrompt, shouldFillPaperPrompt } from './utils/paperPromptStore';

type ReaderTab = {
  id: string;
  name: string;
  pdf: PdfDocument;
  fingerprint: string;
  path: string | null;
  page: number;
  zoom: number | 'fit';
  rotation: number;
};

type Panel = 'pages' | 'outline' | 'bookmarks';

type Toast = { text: string; tone: 'error' | 'info' };

const MAX_FILE_SIZE = 100 * 1024 * 1024;

function basename(path: string) {
  return path.split(/[\\/]/).pop() || path;
}

function flattenOutline(items: PdfOutlineItem[], depth = 0): Array<{ item: PdfOutlineItem; depth: number }> {
  return items.flatMap((item) => [
    { item, depth },
    ...flattenOutline(item.items || [], depth + 1),
  ]);
}

function humanError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export default function App() {
  const [mode, setMode] = useState<'reader' | 'library' | 'chat' | 'records' | 'word'>('library');
  // Word uses an optional ChatGPT dock; ordinary navigation must never leave a native webview over it.
  const [wordChatOpen, setWordChatOpen] = useState(() => localStorage.getItem('research-word-chat-open') === 'true');
  // Persist the ChatGPT workspace layout between application restarts.
  const [chatPdfCollapsed, setChatPdfCollapsed] = useState(() => localStorage.getItem('research-chat-pdf-collapsed') === 'true');
  const [chatPromptRequestId, setChatPromptRequestId] = useState(0);
  const [chatPromptHandledId, setChatPromptHandledId] = useState(0);
  const [chatPromptPendingTabId, setChatPromptPendingTabId] = useState<string | null>(null);
  const [chatRatio, setChatRatio] = useState(() => {
    const stored = Number(localStorage.getItem('research-chat-panel-ratio'));
    return Number.isFinite(stored) && stored >= 0.26 && stored <= 0.68 ? stored : 0.43;
  });
  const [chatResizing, setChatResizing] = useState(false);
  const chatSplitRef = useRef<HTMLDivElement>(null);
  const [pendingClip, setPendingClip] = useState<ClipDraftSeed | null>(null);
  const [pendingWordInsert, setPendingWordInsert] = useState<WordInsert|null>(null);
  // A prompt belongs to its own open PDF, not to a global ChatGPT pane.
  const [chatPromptByTab, setChatPromptByTab] = useState<Record<string, string>>({});
  const [contextMenu, setContextMenu] = useState<{x:number;y:number}|null>(null);
  const [translationDraft, setTranslationDraft] = useState<{source:string;label:string}|null>(null);
  const [tabs, setTabs] = useState<ReaderTab[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [panel, setPanel] = useState<Panel>('pages');
  const [outline, setOutline] = useState<PdfOutlineItem[]>([]);
  const [bookmarks, setBookmarks] = useState<number[]>([]);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [annotationsOpen, setAnnotationsOpen] = useState(true);
  const [selectedAnnotationId, setSelectedAnnotationId] = useState<string | null>(null);
  const [pendingSelection, setPendingSelection] = useState<SelectionDraft | null>(null);
  const [selectionKind, setSelectionKind] = useState<AnnotationKind>('highlight');
  const [restoring, setRestoring] = useState(false);
  const saveTimers = useRef<Map<string, number>>(new Map());
  const [query, setQuery] = useState('');
  const [matches, setMatches] = useState<Array<{ page: number; hits: number }>>([]);
  const [searching, setSearching] = useState(false);
  const [searchShown, setSearchShown] = useState(false);
  const [pageInput, setPageInput] = useState('1');
  const [darkMode, setDarkMode] = useState(() => localStorage.getItem('reader-theme') === 'dark');
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState<Toast | null>(null);
  const [width, setWidth] = useState(900);
  const fileInput = useRef<HTMLInputElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const tabRef = useRef<ReaderTab[]>([]);

  const isChatDockVisible = mode === 'chat' || (mode === 'word' && wordChatOpen);
  useEffect(() => { localStorage.setItem('research-chat-panel-ratio', String(chatRatio)); }, [chatRatio]);
  useEffect(() => { localStorage.setItem('research-chat-pdf-collapsed', String(chatPdfCollapsed)); }, [chatPdfCollapsed]);
  useEffect(() => { localStorage.setItem('research-word-chat-open', String(wordChatOpen)); }, [wordChatOpen]);
  // Keyboard handlers only apply to the app's trusted main view; a native child
  // WebView2 has its own focus and must not receive injected keyboard hooks.
  useEffect(() => {
    if (mode !== 'chat' && mode !== 'word') return;
    const onShortcut = (event: KeyboardEvent) => {
      if (event.repeat || !event.ctrlKey || !event.shiftKey || event.altKey || event.metaKey) return;
      if (!['KeyL', 'KeyC', 'KeyR'].includes(event.code)) return;
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || Boolean(target.closest('input, textarea, select, [contenteditable="true"], [role="textbox"]')))) return;
      if (event.code === 'KeyL' && mode !== 'chat') return;
      if (event.code === 'KeyR' && (mode !== 'chat' || chatPdfCollapsed)) return;
      event.preventDefault();
      if (event.code === 'KeyL') setChatPdfCollapsed(value => !value);
      if (event.code === 'KeyR') setChatRatio(0.43);
      if (event.code === 'KeyC') {
        if (mode === 'word') setWordChatOpen(value => !value);
        else setMode('reader');
      }
    };
    window.addEventListener('keydown', onShortcut);
    return () => window.removeEventListener('keydown', onShortcut);
  }, [mode, chatPdfCollapsed]);
  useEffect(() => { setChatResizing(false); }, [mode, wordChatOpen, chatPdfCollapsed]);

  // Recover from orphaned child WebView2 instances created by older versions or a failed close.
  // This operates only on this app's own child labels, never on the trusted main view.
  useEffect(() => {
    if (!isTauri() || isChatDockVisible) return;
    let cancelled = false;
    void import('@tauri-apps/api/webview').then(async ({getAllWebviews}) => {
      const views = await getAllWebviews();
      if (cancelled) return;
      await Promise.all(views.filter(v => v.label.startsWith('chatgpt-embedded-')).map(async view => {
        try { await view.hide(); } catch (error) { console.warn('Hide orphaned ChatGPT webview:', error); }
        try { await view.close(); } catch (error) { console.warn('Close orphaned ChatGPT webview:', error); }
      }));
    }).catch(error => console.warn('Inspect embedded ChatGPT webviews:', error));
    return () => { cancelled = true; };
  }, [isChatDockVisible]);

  const startChatResize = (event: UIPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    setChatResizing(true);
  };
  const moveChatResize = (event: UIPointerEvent<HTMLDivElement>) => {
    if (!chatResizing || !chatSplitRef.current) return;
    const bounds = chatSplitRef.current.getBoundingClientRect();
    const sidebarWidth = mode === 'chat' && sidebarOpen && window.innerWidth > 1200 ? 188 : 0;
    const available = bounds.width - sidebarWidth - 10;
    if (available < 570) return;
    const desired = bounds.right - event.clientX;
    const lower = 300;
    const upper = Math.min(available - 270, available * 0.70);
    const adjusted = Math.max(lower, Math.min(upper, desired));
    setChatRatio(adjusted / bounds.width);
  };
  const endChatResize = (event: UIPointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    setChatResizing(false);
  };
  const nudgeChatWidth = (direction: -1 | 1) => setChatRatio(v => Math.max(.26, Math.min(.68, v + direction * .025)));
  const chatResizeHandle = <div className="chat-resize-divider" role="separator" tabIndex={0}
    aria-label="拖动以调整左侧编辑器与 ChatGPT 的宽度" aria-orientation="vertical"
    aria-valuenow={Math.round(chatRatio * 100)} aria-valuemin={26} aria-valuemax={68}
    title="拖动调整左右面板宽度；双击或 Ctrl+Shift+R 恢复默认；方向键微调"
    onPointerDown={startChatResize} onPointerMove={moveChatResize} onPointerUp={endChatResize}
    onPointerCancel={endChatResize} onLostPointerCapture={() => setChatResizing(false)}
    onDoubleClick={() => setChatRatio(.43)}
    onKeyDown={event => { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); nudgeChatWidth(event.key === 'ArrowLeft' ? 1 : -1); } }}>
    <span aria-hidden="true" className="chat-resize-grip"/>
  </div>;

  const active = useMemo(() => tabs.find((tab) => tab.id === activeId) || null, [tabs, activeId]);
  const chatPrompt = currentPaperPrompt(chatPromptByTab, activeId);
  const preparedPromptRequestId = shouldFillPaperPrompt(activeId, chatPromptPendingTabId, chatPromptRequestId, chatPromptHandledId, chatPrompt) ? chatPromptRequestId : 0;
  // Switching PDF tabs cancels deferred prompt insertion from the previous paper.
  useEffect(() => { setChatPromptPendingTabId(null); }, [activeId]);
  tabRef.current = tabs;

  const notify = useCallback((text: string, tone: Toast['tone'] = 'info') => {
    setToast({ text, tone });
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 5200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    document.documentElement.dataset.theme = darkMode ? 'dark' : 'light';
    localStorage.setItem('reader-theme', darkMode ? 'dark' : 'light');
  }, [darkMode]);

  useEffect(() => {
    const element = viewportRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setWidth(element.clientWidth));
    observer.observe(element);
    setWidth(element.clientWidth);
    return () => observer.disconnect();
  }, [mode, chatPdfCollapsed]);

  const readInput = useCallback(async (name: string, bytes: Uint8Array, path: string | null = null) => {
    if (bytes.byteLength > MAX_FILE_SIZE) throw new Error(`${name} 超过 V0.3 的 100 MB 单文件限制`);
    if (bytes.byteLength < 5 || String.fromCharCode(...bytes.slice(0, 5)) !== '%PDF-') {
      throw new Error(`${name} 不是有效的 PDF 文件（文件头缺少 %PDF-）`);
    }
    const pdf = await openPdf(bytes);
    const fingerprint = pdf.fingerprints[0] || crypto.randomUUID();
    // Existing document locations and reading progress are indexed by PDF fingerprint.
    let previous = null;
    try { previous = await getDocument(fingerprint); }
    catch (error) { notify(`阅读记录读取失败：${humanError(error)}`, 'error'); }
    const tab: ReaderTab = {
      id: crypto.randomUUID(), name, pdf, fingerprint,
      path: path || previous?.path || null,
      page: Math.min(pdf.numPages, Math.max(1, previous?.page || 1)),
      zoom: previous?.zoom || 'fit', rotation: previous?.rotation || 0,
    };
    setTabs((old) => [...old, tab]);
    setActiveId(tab.id);
    // Restoring recent files does not interrupt the library view.
    return tab;
  }, [notify]);

  const openBrowserFiles = useCallback(async (files: FileList | File[]) => {
    setBusy(true);
    try {
      for (const file of Array.from(files)) {
        try {
          if (!file.name.toLowerCase().endsWith('.pdf')) throw new Error('只能打开 PDF');
          if (file.size > MAX_FILE_SIZE) throw new Error('文件超过 100 MB');
          await readInput(file.name, new Uint8Array(await file.arrayBuffer()));
        } catch (error) {
          notify(`${file.name}：${humanError(error)}`, 'error');
        }
      }
    } finally {
      setBusy(false);
      if (fileInput.current) fileInput.current.value = '';
    }
  }, [notify, readInput]);

  const openFiles = useCallback(async () => {
    setMode('reader');
    if (!isTauri()) {
      fileInput.current?.click();
      return;
    }
    setBusy(true);
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const { readFile } = await import('@tauri-apps/plugin-fs');
      const paths = await open({ multiple: true, directory: false, filters: [{ name: 'PDF Documents', extensions: ['pdf'] }] });
      if (!paths) return;
      for (const path of Array.isArray(paths) ? paths : [paths]) {
        try {
          await readInput(basename(path), await readFile(path), path);
        } catch (error) {
          notify(`${basename(path)}：${humanError(error)}`, 'error');
        }
      }
    } catch (error) {
      notify(`文件打开失败：${humanError(error)}`, 'error');
    } finally {
      setBusy(false);
    }
  }, [notify, readInput]);

  const openFromLibrary = useCallback(async (path:string, targetPage?:number | null) => {
    const existing = tabRef.current.find((tab) => tab.path === path);
    if (existing) {
      if (targetPage && Number.isFinite(targetPage)) setTabs(old => old.map(tab => tab.id === existing.id ? { ...tab, page: Math.max(1, Math.min(tab.pdf.numPages, targetPage)) } : tab));
      setActiveId(existing.id); setMode('reader'); return;
    }
    setBusy(true);
    try {
      const bytes = await import('@tauri-apps/api/core').then(m => m.invoke<number[]>('read_recent_pdf', { path }));
      const tab = await readInput(basename(path), new Uint8Array(bytes), path);
      if (targetPage && Number.isFinite(targetPage)) setTabs(old => old.map(item => item.id === tab.id ? { ...item, page: Math.max(1, Math.min(item.pdf.numPages, targetPage)) } : item));
      setMode('reader');
    } catch (error) {notify(`无法打开文献 PDF：${humanError(error)}`, 'error');}
    finally {setBusy(false);}
  }, [readInput, notify]);

  // Native desktop app automatically restores recently opened local files and their last page.
  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    setRestoring(true);
    (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const recent = (await listDocuments()).filter((doc) => Boolean(doc.path)).slice(0, 4).reverse();
        for (const doc of recent) {
          if (cancelled) break;
          try {
            const bytes = await invoke<number[]>('read_recent_pdf', { path: doc.path });
            if (cancelled) break;
            await readInput(doc.name, new Uint8Array(bytes), doc.path);
          } catch (error) {
            if (!cancelled) notify(`未能恢复 ${doc.name}：${humanError(error)}`, 'error');
          }
        }
      } catch (error) { if (!cancelled) notify(`恢复阅读记录失败：${humanError(error)}`, 'error'); }
      finally { if (!cancelled) setRestoring(false); }
    })();
    return () => { cancelled = true; };
  }, [readInput, notify]);

  // Changing page, zoom or rotation is persisted, not tied to the life of a browser tab.
  useEffect(() => {
    if (!active || restoring) return;
    const timer = window.setTimeout(() => {
      void saveDocument({ fingerprint: active.fingerprint, name: active.name, path: active.path,
        page: active.page, zoom: active.zoom, rotation: active.rotation,
        lastOpened: new Date().toISOString(),
      }).catch((error) => notify(`阅读进度保存失败：${humanError(error)}`, 'error'));
    }, 320);
    return () => window.clearTimeout(timer);
  }, [active?.id, active?.page, active?.zoom, active?.rotation, active?.path, restoring, notify]);

  useEffect(() => {
    if (!active) { setAnnotations([]); setSelectedAnnotationId(null); return; }
    let cancelled = false;
    setAnnotations([]);
    setSelectedAnnotationId(null);
    getAnnotations(active.fingerprint)
      .then((items) => { if (!cancelled) setAnnotations(items); })
      .catch((error) => { if (!cancelled) notify(`批注读取失败：${humanError(error)}`, 'error'); });
    return () => { cancelled = true; };
  }, [active?.fingerprint, notify]);

  useEffect(() => { setPendingSelection(null); setContextMenu(null); }, [active?.id, active?.page, active?.zoom, active?.rotation]);

  const closeTab = useCallback((id: string) => {
    const victim = tabRef.current.find((tab) => tab.id === id);
    setTabs((existing) => existing.filter((tab) => tab.id !== id));
    setChatPromptByTab(current => removePaperPrompt(current, id));
    setChatPromptPendingTabId(current => current === id ? null : current);
    if (victim) {
      // PDF.js worker references are released when the document is closed.
      window.setTimeout(() => void victim.pdf.cleanup(), 100);
    }
    setActiveId((current) => {
      if (current !== id) return current;
      const remaining = tabRef.current.filter((tab) => tab.id !== id);
      return remaining.length ? remaining[remaining.length - 1].id : null;
    });
  }, []);

  const updateActive = useCallback((changes: Partial<ReaderTab>) => {
    if (!activeId) return;
    setTabs((existing) => existing.map((tab) => tab.id === activeId ? { ...tab, ...changes } : tab));
  }, [activeId]);

  const goToPage = useCallback((target: number) => {
    if (!active) return;
    const clamped = Math.max(1, Math.min(active.pdf.numPages, Math.round(target)));
    updateActive({ page: clamped });
  }, [active, updateActive]);

  useEffect(() => {
    setQuery('');
    setMatches([]);
    setSearchShown(false);
    setOutline([]);
    if (!active) return;
    setPageInput(String(active.page));
    setBookmarks(getBookmarks(active.fingerprint));
    let cancelled = false;
    active.pdf.getOutline()
      .then((items) => { if (!cancelled) setOutline((items || []) as unknown as PdfOutlineItem[]); })
      .catch(() => {});
    return () => { cancelled = true; };
    // Only reset per-document navigation state when the active document switches.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeId]);

  useEffect(() => {
    if (active) setPageInput(String(active.page));
  }, [active?.page]);

  useEffect(() => {
    if (!active || !query.trim()) {
      setMatches([]);
      setSearching(false);
      return;
    }
    let cancelled = false;
    setSearching(true);
    const timer = window.setTimeout(() => {
      searchPdf(active.pdf, query)
        .then((result) => { if (!cancelled) setMatches(result); })
        .catch((error) => { if (!cancelled) notify(`搜索失败：${humanError(error)}`, 'error'); })
        .finally(() => { if (!cancelled) setSearching(false); });
    }, 280);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [active?.id, query, notify]);

  const findNext = useCallback((backward = false) => {
    if (!active || matches.length === 0) return;
    const sorted = matches.map(({ page }) => page);
    const candidate = backward
      ? [...sorted].reverse().find((page) => page < active.page) ?? sorted[sorted.length - 1]
      : sorted.find((page) => page > active.page) ?? sorted[0];
    goToPage(candidate);
  }, [active, matches, goToPage]);

  const toggleCurrentBookmark = useCallback(() => {
    if (!active) return;
    setBookmarks(toggleBookmark(active.fingerprint, active.page));
  }, [active]);

  const createAnnotation = useCallback((color: AnnotationColor) => {
    if (!active || !pendingSelection) return;
    const now = new Date().toISOString();
    const annotation: Annotation = {
      id: crypto.randomUUID(), fingerprint: active.fingerprint,
      page: pendingSelection.page, quote: pendingSelection.quote,
      rects: pendingSelection.rects, kind: selectionKind, color, note: '',
      createdAt: now, updatedAt: now,
    };
    setAnnotations((old) => [...old, annotation]);
    setSelectedAnnotationId(annotation.id);
    setAnnotationsOpen(true);
    setPendingSelection(null);
    window.getSelection()?.removeAllRanges();
    void saveAnnotation(annotation).catch((error) => notify(`批注保存失败：${humanError(error)}`, 'error'));
  }, [active, pendingSelection, selectionKind, notify]);

  const updateNote = useCallback((annotation: Annotation, note: string) => {
    const next = { ...annotation, note, updatedAt: new Date().toISOString() };
    setAnnotations((old) => old.map((item) => item.id === next.id ? next : item));
    const existing = saveTimers.current.get(next.id);
    if (existing !== undefined) window.clearTimeout(existing);
    const timer = window.setTimeout(() => {
      saveTimers.current.delete(next.id);
      void saveAnnotation(next).catch((error) => notify(`笔记保存失败：${humanError(error)}`, 'error'));
    }, 350);
    saveTimers.current.set(next.id, timer);
  }, [notify]);

  const removeAnnotation = useCallback((annotation: Annotation) => {
    const timer = saveTimers.current.get(annotation.id);
    if (timer !== undefined) window.clearTimeout(timer);
    saveTimers.current.delete(annotation.id);
    setAnnotations((old) => old.filter((ann) => ann.id !== annotation.id));
    setSelectedAnnotationId((current) => current === annotation.id ? null : current);
    void deleteAnnotation(annotation.id).catch((error) => notify(`删除批注失败：${humanError(error)}`, 'error'));
  }, [notify]);

  const selectAnnotation = useCallback((annotation: Annotation) => {
    setSelectedAnnotationId(annotation.id);
    setAnnotationsOpen(true);
    goToPage(annotation.page);
  }, [goToPage]);

  const exportAnnotations = useCallback(async () => {
    if (!active) return;
    const data = markdownForAnnotations(active.name, annotations);
    const filename = active.name.replace(/\.pdf$/i, '').replace(/[\\/:*?"<>|]/g, '_') + '-批注.md';
    try {
      if (isTauri()) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const path = await save({ defaultPath: filename, filters: [{ name: 'Markdown', extensions: ['md'] }] });
        if (!path) return;
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('write_markdown', { path, content: data });
      } else {
        const blob = new Blob([data], { type: 'text/markdown;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a'); a.href = url; a.download = filename; a.click();
        window.setTimeout(() => URL.revokeObjectURL(url), 3000);
      }
      notify('批注已导出为 Markdown');
    } catch (error) { notify(`导出失败：${humanError(error)}`, 'error'); }
  }, [active, annotations, notify]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const modifier = event.ctrlKey || event.metaKey;
      if (modifier && event.key.toLowerCase() === 'o') {
        event.preventDefault();
        void openFiles();
        return;
      }
      if ((mode === 'reader' || mode === 'chat') && modifier && event.key.toLowerCase() === 'f') {
        event.preventDefault();
        setSearchShown(true);
        window.requestAnimationFrame(() => searchInput.current?.focus());
        return;
      }
      if (mode !== 'reader' && mode !== 'chat') return;
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
      if (event.key === 'ArrowRight' || event.key === 'PageDown') goToPage((active?.page || 1) + 1);
      if (event.key === 'ArrowLeft' || event.key === 'PageUp') goToPage((active?.page || 1) - 1);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [active?.page, goToPage, openFiles, mode]);

  const promptForSelection = useCallback((kind: ResearchPromptKind) => {
    if (!active || !pendingSelection) return;
    const prompt = buildResearchPrompt(kind, pendingSelection.quote, active.name, pendingSelection.page);
    setChatPromptByTab(current => replacePaperPrompt(current, active.id, prompt));
    setChatPromptPendingTabId(active.id);
    setChatPromptRequestId(id => id + 1);
    setChatPdfCollapsed(false);
    setPendingSelection(null);
    setContextMenu(null);
    setMode('chat');
    notify('科研提示词已生成，正在尝试填入 ChatGPT；请检查后手动发送。');
  }, [active, pendingSelection, notify]);

  const insertSelectionIntoWord = useCallback(() => {
    if(!active||!pendingSelection)return;
    setPendingWordInsert({id:crypto.randomUUID(),body:pendingSelection.quote,title:`${active.name} · 第 ${pendingSelection.page} 页摘录`});
    setPendingSelection(null);setContextMenu(null);setMode('word');
  },[active,pendingSelection]);

  const saveSelectionAsResearch = useCallback(() => {
    if (!active || !pendingSelection) return;
    setPendingClip({ body: pendingSelection.quote, title: `${active.name} · 第 ${pendingSelection.page} 页摘录`,
      fingerprint: active.fingerprint, page: pendingSelection.page });
    setPendingSelection(null);
    setContextMenu(null);
    setMode('records');
  }, [active, pendingSelection]);

  // A live, consistent SQLite snapshot: VACUUM INTO uses SQLite's own snapshot mechanism.
  const backupData = useCallback(async () => {
    if (!isTauri()) { notify('数据库备份仅支持安装后的 Windows 软件', 'error'); return; }
    try {
      const [{ save }, { getDatabase }] = await Promise.all([
        import('@tauri-apps/plugin-dialog'), import('./utils/readerStore')]);
      const filename = `research-studio-${new Date().toISOString().slice(0,10)}.sqlite`;
      const path = await save({ defaultPath: filename, filters: [{name:'SQLite 完整备份',extensions:['sqlite']}] });
      if (!path) return;
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('backup_prepare_target', {path});
      const db = await getDatabase();
      await db.execute('VACUUM INTO $1', [path]);
      notify('完整数据库备份已生成。原始 PDF 文件仍需单独保留。');
    } catch (error) { notify(`备份失败：${humanError(error)}`, 'error'); }
  }, [notify]);

  const restoreData = useCallback(async () => {
    if (!isTauri()) { notify('数据库恢复仅支持安装后的 Windows 软件', 'error'); return; }
    if (!window.confirm('恢复将替换当前数据库中的文献、批注、科研记录和译文缓存。请先备份当前数据，确认继续选择备份文件吗？')) return;
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const path = await open({multiple:false,directory:false,filters:[{name:'SQLite 数据库备份',extensions:['sqlite']}]});
      if (typeof path !== 'string') return;
      if (!window.confirm('最后确认：下次重新启动软件将用所选备份替换全部本地数据库记录。当前修改将无法保留。继续吗？')) return;
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('stage_database_restore', {path});
      notify('备份已校验并安排恢复。请完全退出软件，然后重新启动以完成恢复。');
    } catch (error) { notify(`恢复准备失败：${humanError(error)}`, 'error'); }
  }, [notify]);

  const selectedHitCount = matches.reduce((sum, item) => sum + item.hits, 0);
  const zoomLabel = active?.zoom === 'fit' ? '适应宽度' : `${Math.round((active?.zoom || 1) * 100)}%`;
  const applyZoom = (direction: -1 | 1) => {
    if (!active) return;
    const current = active.zoom === 'fit' ? 1 : active.zoom;
    updateActive({ zoom: Math.min(3, Math.max(.5, Math.round((current + direction * .15) * 100) / 100)) });
  };

  return (
    <div className="app-shell" onDragOver={(event) => event.preventDefault()} onDrop={(event) => {
      event.preventDefault();
      if (mode === 'reader' && event.dataTransfer.files.length) void openBrowserFiles(event.dataTransfer.files);
    }}>
      <input ref={fileInput} className="hidden" type="file" accept="application/pdf,.pdf" multiple
        onChange={(event) => { if (event.target.files) void openBrowserFiles(event.target.files); }} />
      <header className="app-header">
        <div className="brand"><div className="brand-icon"><BookOpen size={19}/></div><div><strong>Research PDF</strong><small>AI RESEARCH STUDIO · V0.5.9</small></div></div>
        <div className="header-actions"><div className="app-mode-switch">
          <button className={mode === 'library' ? 'active' : ''} onClick={() => setMode('library')}><LibraryBig size={16}/> 文献库</button>
          <button className={mode === 'reader' ? 'active' : ''} onClick={() => setMode('reader')}><BookOpen size={16}/> PDF 阅读</button>
          <button className={mode === 'chat' ? 'active' : ''} onClick={() => setMode('chat')}><Globe size={16}/> ChatGPT 网页</button>
          <button className={mode === 'records' ? 'active' : ''} onClick={() => setMode('records')}><NotebookPen size={16}/> 科研记录</button>
          <button className={mode === 'word' ? 'active' : ''} onClick={() => setMode('word')}><FilePenLine size={16}/> Word 论文</button>
        {mode === 'word' && <button className={wordChatOpen ? 'active' : ''} aria-pressed={wordChatOpen} title="在 Word 右侧打开或关闭 ChatGPT 侧栏" onClick={() => setWordChatOpen(v => !v)}><MessageSquareText size={16}/> {wordChatOpen ? '关闭 ChatGPT 侧栏' : '打开 ChatGPT 侧栏'}</button>}
        </div><button className="icon-button" title="完整数据库备份（含文献库、批注和科研记录，不包含原始 PDF）" onClick={() => void backupData()}><DatabaseBackup size={17}/></button><button className="icon-button" title="从 SQLite 备份恢复（需重启）" onClick={() => void restoreData()}><DatabaseZap size={17}/></button><button className="button translator-settings-shortcut" title="翻译配置和术语词典" onClick={() => setTranslationDraft({source:'',label:'翻译设置'})}><Languages size={15}/> 翻译设置</button><span className="mode-label">本地科研工作站</span><button className="icon-button" title="切换主题" onClick={() => setDarkMode((old) => !old)}>{darkMode ? <Sun size={18}/> : <Moon size={18}/>}</button></div>
      </header>
      {(mode === 'reader' || mode === 'chat') && <>
      {!(mode === 'chat' && chatPdfCollapsed) && <>
      <div className="tab-row">
        {tabs.map((tab) => <button key={tab.id} className={`document-tab ${tab.id === activeId ? 'active' : ''}`} onClick={() => setActiveId(tab.id)} title={tab.name}>
          <FileText size={15}/><span>{tab.name}</span><span className="tab-close" role="button" tabIndex={0} aria-label={`关闭 ${tab.name}`} onClick={(event) => {event.stopPropagation();closeTab(tab.id);}} onKeyDown={(event) => {if(event.key === 'Enter'){event.stopPropagation();closeTab(tab.id);}}}><X size={13}/></span>
        </button>)}
        <button className="tab-plus" title="打开 PDF" disabled={busy} onClick={() => void openFiles()}><Plus size={18}/></button>
      </div>
      <div className="toolbar">
        <div className="toolbar-left">
          <button className="icon-button" title={sidebarOpen ? '收起左侧栏' : '展开左侧栏'} onClick={() => setSidebarOpen((x) => !x)}>{sidebarOpen ? <PanelLeftClose size={18}/> : <PanelLeftOpen size={18}/>}</button>
          <span className="v-separator"/>
          <button className="button primary" disabled={busy} onClick={() => void openFiles()}><FolderOpen size={16}/><span>{busy ? '正在打开…' : '打开 PDF'}</span></button>
        </div>
        <div className="toolbar-middle">
          <button className="icon-button" disabled={!active || active.page <= 1} title="上一页" onClick={() => goToPage((active?.page || 1) - 1)}><ChevronLeft size={18}/></button>
          <input className="page-number" aria-label="当前页码" disabled={!active} value={pageInput}
            onChange={(event) => setPageInput(event.target.value)} onBlur={() => active && setPageInput(String(active.page))}
            onKeyDown={(event) => { if (event.key === 'Enter') goToPage(Number(pageInput) || 1); }} />
          <span className="total-pages">/ {active?.pdf.numPages || 0}</span>
          <button className="icon-button" disabled={!active || active.page >= active.pdf.numPages} title="下一页" onClick={() => goToPage((active?.page || 1) + 1)}><ChevronRight size={18}/></button>
          <span className="v-separator"/>
          <button className="icon-button" title="缩小" disabled={!active} onClick={() => applyZoom(-1)}><Minus size={17}/></button>
          <button className="zoom-fit" title="切换适应宽度 / 100%" disabled={!active} onClick={() => updateActive({ zoom: active?.zoom === 'fit' ? 1 : 'fit' })}>{zoomLabel}<ChevronDown size={13}/></button>
          <button className="icon-button" title="放大" disabled={!active} onClick={() => applyZoom(1)}><Plus size={17}/></button>
          <span className="v-separator"/>
          <button className="icon-button" title="顺时针旋转" disabled={!active} onClick={() => updateActive({ rotation: ((active?.rotation || 0) + 90) % 360 })}><RotateCw size={17}/></button>
        </div>
        <div className="toolbar-right">
          <button className={`icon-button ${searchShown ? 'selected-button' : ''}`} title="全文搜索 Ctrl+F" disabled={!active} onClick={() => { setSearchShown(!searchShown); window.setTimeout(() => searchInput.current?.focus(), 0); }}><Search size={18}/></button>
          <button className={`icon-button ${active && bookmarks.includes(active.page) ? 'selected-button' : ''}`} title="添加/取消本页书签" disabled={!active} onClick={toggleCurrentBookmark}><BookMarked size={18}/></button>
          <button className={`icon-button ${annotationsOpen ? 'selected-button' : ''}`} title="展开/收起论文批注" disabled={!active} onClick={() => setAnnotationsOpen((v) => !v)}><MessageSquareText size={18}/></button>
        </div>
      </div>
      </>}
      {!(mode === 'chat' && chatPdfCollapsed) && active && searchShown && <div className="search-strip">
        <Search size={17}/><input ref={searchInput} placeholder="搜索整篇 PDF 中的文字…" value={query} onChange={(event) => setQuery(event.target.value)} />
        <span className="search-counter">{searching ? '搜索中…' : query.trim() ? `${matches.length} 页 / ${selectedHitCount} 处` : '输入关键词'}</span>
        <button className="icon-button" disabled={!matches.length} title="上一匹配页" onClick={() => findNext(true)}><ChevronLeft size={16}/></button>
        <button className="icon-button" disabled={!matches.length} title="下一匹配页" onClick={() => findNext()}><ChevronRight size={16}/></button>
        <button className="icon-button" onClick={() => { setSearchShown(false); setQuery(''); }}><X size={17}/></button>
      </div>}
      <div className={`workspace ${mode === 'chat' ? `chat-split-workspace ${chatPdfCollapsed ? 'chat-pdf-collapsed' : ''}` : ''}`} ref={mode === 'chat' ? chatSplitRef : undefined} style={mode === 'chat' ? {'--chat-panel-width': `${chatRatio * 100}%`} as CSSProperties : undefined}>
        {sidebarOpen && !(mode === 'chat' && chatPdfCollapsed) && <aside className="sidebar">
          <div className="sidebar-tabs">
            <button className={panel === 'pages' ? 'active' : ''} title="缩略图" onClick={() => setPanel('pages')}><FileText size={16}/> 页面</button>
            <button className={panel === 'outline' ? 'active' : ''} title="大纲" onClick={() => setPanel('outline')}><List size={16}/> 大纲</button>
            <button className={panel === 'bookmarks' ? 'active' : ''} title="书签" onClick={() => setPanel('bookmarks')}><BookMarked size={16}/> 书签</button>
          </div>
          {!active && <div className="sidebar-empty"><FileSearch2 size={28}/><span>打开 PDF 后显示页面目录</span></div>}
          {active && panel === 'pages' && <div className="thumb-scroll">
            {Array.from({ length: active.pdf.numPages }, (_, index) => <Thumbnail key={`${active.id}-${index}`} pdf={active.pdf} page={index + 1} selected={index + 1 === active.page} onClick={() => goToPage(index + 1)}/>)}
          </div>}
          {active && panel === 'outline' && <div className="outline-list">
            {outline.length === 0 ? <p className="empty-hint">此 PDF 没有内置章节大纲</p> : flattenOutline(outline).map(({ item, depth }, index) => <button key={index} className="outline-item" style={{ paddingLeft: 12 + Math.min(depth, 5) * 13 }} onClick={() => void outlineTargetPage(active.pdf, item.dest).then((page) => page && goToPage(page))}><Hash size={13}/><span>{item.title}</span></button>)}
          </div>}
          {active && panel === 'bookmarks' && <div className="bookmarks-list">
            {bookmarks.length === 0 ? <p className="empty-hint">尚无书签。使用工具栏书签按钮收藏当前页。</p> : bookmarks.map((page) => <button className="bookmark-row" key={page} onClick={() => goToPage(page)}><BookMarked size={15}/> 第 {page} 页 <ChevronRight size={14}/></button>)}
          </div>}
          <div className="sidebar-bottom"><Clock3 size={14}/> 全部文件仅在本机处理</div>
        </aside>}
        {!(mode === 'chat' && chatPdfCollapsed) && <main className="reader-viewport" ref={viewportRef}>
          {active ? <>
            <div className="document-context"><span>{active.name}</span><span>第 {active.page} 页，共 {active.pdf.numPages} 页</span></div>
            <PdfPage pdf={active.pdf} pageNumber={active.page} zoom={active.zoom} rotation={active.rotation}
              query={query} availableWidth={width}
              annotations={annotations.filter((ann) => ann.page === active.page)}
              selectedAnnotationId={selectedAnnotationId}
              onSelectText={(selection) => { setPendingSelection(selection); setContextMenu(null); }}
              onSelectionContextMenu={(event) => { setContextMenu({x:event.clientX,y:event.clientY}); }}
              onActivateAnnotation={(id) => { setSelectedAnnotationId(id); setAnnotationsOpen(true); }}/>
            <div className="page-bottom-note">鼠标选择文字后可翻译、高亮、下划线、删除线并添加论文笔记</div>
          </> : <div className="welcome-state">
            <div className="welcome-art"><div className="paper-card back"/><div className="paper-card front"><span className="paper-line title"/><span className="paper-line"/><span className="paper-line short"/><span className="paper-graph"><i/><i/><i/><i/></span><span className="paper-line"/></div><span className="floating-icon"><ZoomIn size={24}/></span></div>
            <h1>开始阅读你的研究文献</h1>
            <p>一个专注于阅读体验的本地 PDF 工作空间。<br/>打开论文、检索内容、浏览章节，并保存重要页面。</p>
            <button className="button primary big" onClick={() => void openFiles()}><FilePlus2 size={18}/> 打开 PDF 文件</button>
            <div className="drop-hint">或将 PDF 文件拖入此窗口 · 支持同时打开多个文件</div>
            <div className="welcome-features"><span><FileText size={16}/> 清晰的文本阅读</span><span><Search size={16}/> 全文检索</span><span><SidebarOpen size={16}/> 多标签管理</span></div>
          </div>}
        </main>}
        {mode === 'chat' && <>{!chatPdfCollapsed && chatResizeHandle}<ChatBrowserDock key="chat-dock" activePaperId={active?.id || null} activePaperName={active?.name || null} preparedPrompt={chatPrompt} preparedPromptRequestId={preparedPromptRequestId} onPromptHandled={id => setChatPromptHandledId(v => Math.max(v,id))} pdfCollapsed={chatPdfCollapsed} onTogglePdf={() => setChatPdfCollapsed(v => !v)} resizing={chatResizing} onClose={() => setMode('reader')} onNotify={notify} activeFingerprint={active?.fingerprint || null} activePage={active?.page || null} onCaptureText={(text) => {
          setPendingClip({body: text, fingerprint: active?.fingerprint || null, page: active?.page || null});
          setMode('records');
        }}/></>}
        {mode === 'reader' && active && annotationsOpen && <AnnotationPanel
          title={active.name} annotations={annotations} selectedId={selectedAnnotationId}
          onSelect={selectAnnotation} onUpdateNote={updateNote}
          onDelete={removeAnnotation} onExport={() => void exportAnnotations()}
          onClose={() => setAnnotationsOpen(false)} />}
      </div>
      {!(mode === 'chat' && chatPdfCollapsed) && active && pendingSelection && <div className="selection-actions" role="toolbar"
        aria-label="对选中文字进行批注或翻译"
        onMouseDown={(event) => event.preventDefault()}
        style={{ left: Math.max(12, Math.min(window.innerWidth - 280, pendingSelection.clientX - 105)),
          top: Math.max(155, Math.min(window.innerHeight - 100, pendingSelection.clientY + 10)) }}>
        <select aria-label="批注类型" value={selectionKind}
          onChange={(event) => setSelectionKind(event.target.value as AnnotationKind)}>
          <option value="highlight">高亮</option><option value="underline">下划线</option>
          <option value="strike">删除线</option>
        </select>
        {(['yellow', 'green', 'blue', 'pink'] as AnnotationColor[]).map((color) =>
          <button key={color} className="selection-color" title={`${color} 批注`}
            style={{ background: COLOR_HEX[color] }} onClick={() => createAnnotation(color)}><Highlighter size={13}/></button>)}
        <button className="selection-translate" title="使用 Azure 翻译选中文字" onClick={() => {
          setTranslationDraft({source:pendingSelection.quote,label:`${active.name} · 第 ${pendingSelection.page} 页`});
          setPendingSelection(null);
        }}><Languages size={14}/> 翻译</button>
        <button className="selection-translate" title="发送到内置 ChatGPT，保留论文页码来源" onClick={() => promptForSelection('explain')}><Sparkles size={14}/> ChatGPT 分析</button>
        <button className="selection-translate" title="插入选中文字到 Word 编辑器" onClick={insertSelectionIntoWord}><FilePenLine size={14}/> 插入 Word</button>
        <button className="selection-translate" title="将 PDF 摘录保存到科研记录" onClick={saveSelectionAsResearch}><NotebookPen size={14}/> 存科研记录</button>
        <button className="selection-dismiss" title="取消批注" onClick={() => setPendingSelection(null)}><X size={15}/></button>
      </div>}
      {!(mode === 'chat' && chatPdfCollapsed) && active && pendingSelection && contextMenu && <div className="pdf-research-context-menu" role="menu"
        onMouseDown={event => event.preventDefault()}
        style={{left:Math.min(window.innerWidth-260,Math.max(10,contextMenu.x)),top:Math.min(window.innerHeight-330,Math.max(70,contextMenu.y))}}>
        <strong>PDF 科研操作 · 第 {pendingSelection.page} 页</strong>
        <button role="menuitem" onClick={()=>{setTranslationDraft({source:pendingSelection.quote,label:`${active.name} · 第 ${pendingSelection.page} 页`});setContextMenu(null);setPendingSelection(null);}}><Languages size={15}/> 翻译选中内容</button>
        <button role="menuitem" onClick={()=>promptForSelection('explain')}><Sparkles size={15}/> ChatGPT 解释段落</button>
        <button role="menuitem" onClick={()=>promptForSelection('math')}><Sigma size={15}/> 分析数学公式与推导</button>
        <button role="menuitem" onClick={()=>promptForSelection('novelty')}><Lightbulb size={15}/> 研究空白与潜在创新性</button>
        <button role="menuitem" onClick={()=>promptForSelection('python')}><FileText size={15}/> 生成 Python 验证方案</button>
        <button role="menuitem" onClick={saveSelectionAsResearch}><NotebookPen size={15}/> 保存到科研记录</button>
        <button role="menuitem" onClick={insertSelectionIntoWord}><FilePenLine size={15}/> 将选中内容插入 Word</button>
        <button role="menuitem" onClick={()=>{setContextMenu(null);setPendingSelection(null);}}><X size={15}/> 关闭菜单</button>
      </div>}
      </>}
      {mode === 'library' && <LiteratureLibrary onOpenPdf={openFromLibrary} onNotify={notify}/>}
      {mode === 'records' && <ResearchRecords initialDraft={pendingClip} onDraftConsumed={() => setPendingClip(null)} onOpenPdf={openFromLibrary} onNotify={notify} onTranslate={(source,label)=>setTranslationDraft({source,label})} onInsertToWord={(body,title,source)=>{setPendingWordInsert({id:crypto.randomUUID(),body,title,source});setMode('word');}}/>}
      {mode === 'word' && <div ref={wordChatOpen ? chatSplitRef : undefined} className={`word-editor-shell ${wordChatOpen ? 'word-chat-split' : ''}`} style={wordChatOpen ? {'--chat-panel-width': `${chatRatio * 100}%`} as CSSProperties : undefined}>
        <div className="word-editor-content"><WordEditor incoming={pendingWordInsert} onConsumed={()=>setPendingWordInsert(null)} onNotify={notify}/></div>
        {wordChatOpen && <>{chatResizeHandle}<ChatBrowserDock resizing={chatResizing} onClose={() => setWordChatOpen(false)} onNotify={notify} activeFingerprint={active?.fingerprint || null} activePage={active?.page || null} onCaptureText={text => {setPendingClip({body:text});setMode('records');}}/></>}
      </div>}
      {translationDraft && <TranslatorWorkspace source={translationDraft.source} label={translationDraft.label} onClose={() => setTranslationDraft(null)} onNotify={notify}/>}
      <footer className="status-bar"><span><span className="status-dot"/> PDF.js 本地渲染</span><span>{active ? `${active.pdf.numPages} 页 · ${active.name}` : '尚未打开文档'}</span><span>{restoring ? '正在恢复上次阅读…' : 'AI Research PDF Reader V0.5.9'}</span></footer>
      {toast && <div role="alert" className={`toast ${toast.tone}`}><CircleAlert size={17}/>{toast.text}<button title="关闭" onClick={() => setToast(null)}><X size={15}/></button></div>}
    </div>
  );
}
