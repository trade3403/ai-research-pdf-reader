import { useEffect, useRef, useState } from 'react';
import { electronBridge } from '../utils/electronBridge';
import { isTauri } from '@tauri-apps/api/core';
import { ChevronDown, ChevronUp, ClipboardPaste, ExternalLink, Globe, Info, LoaderCircle, PanelLeftClose, PanelLeftOpen, RefreshCw, Save, X } from 'lucide-react';
import { loadLibrary } from '../utils/libraryStore';
import type { LiteratureItem } from '../utils/libraryStore';
import { saveClip, validChatGptSource } from '../utils/researchClips';
import { matchesPhysicalWebviewRect, physicalWebviewGeometryKey, toPhysicalWebviewRect } from '../utils/nativeWebviewGeometry';

const CHATGPT_URL = 'https://chatgpt.com/';
type Props = {
  onCaptureText: (text: string) => void;
  onClose: () => void;
  resizing: boolean;
  preparedPrompt?: string;
  activePaperId?: string | null;
  activePaperName?: string | null;
  preparedPromptRequestId?: number;
  onPromptHandled?: (requestId: number) => void;
  pdfCollapsed?: boolean;
  onTogglePdf?: () => void;
  onNotify: (message: string, tone?: 'error' | 'info') => void;
  activeFingerprint: string | null;
  activePage: number | null;
};

type BrowserState = 'starting' | 'ready' | 'fallback' | 'error';

/**
 * Real native child WebView2 inside the Tauri main window (not an iframe).
 * User-directed prompt insertion is performed only by the native Rust command.
 * No remote Tauri IPC, history scraping, or automatic submission.
 */
export default function ChatBrowserDock({ onCaptureText, onClose, resizing, onNotify, activeFingerprint, activePage, preparedPrompt = '', activePaperId = null, activePaperName = null, preparedPromptRequestId = 0, onPromptHandled, pdfCollapsed = false, onTogglePdf }: Props) {
  const mountRef = useRef<HTMLDivElement>(null);
  const childRef = useRef<import('@tauri-apps/api/webview').Webview | null>(null);
  const [childLabel, setChildLabel] = useState<string | null>(null);
  const [fillStatus, setFillStatus] = useState('');
  const [promptCollapsed, setPromptCollapsed] = useState(
    () => localStorage.getItem('research-chat-prompt-collapsed') === 'true',
  );
  useEffect(() => { localStorage.setItem('research-chat-prompt-collapsed', String(promptCollapsed)); }, [promptCollapsed]);
  useEffect(() => { setFillStatus(''); }, [activePaperId]);
  useEffect(() => { if (preparedPromptRequestId > 0) setPromptCollapsed(false); }, [preparedPromptRequestId]);
  const [filling, setFilling] = useState(false);
  const alignRef = useRef<(() => Promise<boolean>) | null>(null);
  const resizingRef = useRef(resizing);
  const [state, setState] = useState<BrowserState>(isTauri() || electronBridge() ? 'starting' : 'fallback');
  const [details, setDetails] = useState('');
  const [browserKey, setBrowserKey] = useState(0);
  const [reading, setReading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [captureText, setCaptureText] = useState('');
  const [captureTitle, setCaptureTitle] = useState('');
  const [sourceUrl, setSourceUrl] = useState(CHATGPT_URL);
  const [papers, setPapers] = useState<LiteratureItem[]>([]);
  const [paperId, setPaperId] = useState('');
  const [page, setPage] = useState<number | null>(activePage);

  resizingRef.current = resizing;
  useEffect(() => {
    if (!isTauri()) return;
    const view = childRef.current;
    if (!view) return;
    // A native child webview intercepts mouse events, so hide it during splitter drags.
    if (resizing) {
      void view.hide().catch(error => setDetails(`暂时隐藏网页失败：${String(error)}`));
    } else {
      void (async () => {
        const aligned = await alignRef.current?.();
        if (aligned && childRef.current === view) await view.show();
      })().catch(error => setDetails(`恢复网页显示失败：${String(error)}`));
    }
  }, [resizing]);

  useEffect(() => {
    let cancelled = false;
    loadLibrary().then(library => { if (!cancelled) setPapers(library.items); })
      .catch(error => onNotify(`文献库读取失败：${String(error)}`, 'error'));
    return () => { cancelled = true; };
  }, [onNotify]);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    let mounted = false;
    let child: import('@tauri-apps/api/webview').Webview | null = null;
    let observer: ResizeObserver | null = null;
    let scheduled = 0;
    let geometryTimer: ReturnType<typeof setInterval> | null = null;
    let lastGeometry = '';
    let busy: Promise<boolean> = Promise.resolve(false);
    const viewport = window.visualViewport;
    // Native child WebView2 is outside CSS layout. A flex sibling can move its
    // left edge without resizing the observed viewport; track position AND size.
    const readGeometry = () => {
      const rect = mountRef.current?.getBoundingClientRect();
      if (!rect || rect.width < 80 || rect.height < 80) return null;
      return {
        x: Math.max(0, rect.left),
        y: Math.max(0, rect.top),
        width: Math.max(1, rect.width),
        height: Math.max(1, rect.height),
      };
    };

    const align = (): Promise<boolean> => {
      if (!child || !mounted || !mountRef.current || cancelled || resizingRef.current) return busy;
      const target = child;
      busy = busy.then(async () => {
        if (cancelled || childRef.current !== target || resizingRef.current) return false;
        const rect = readGeometry();
        if (!rect) return false;

        // Important: CSS geometry is in CSS pixels. At 150% Windows DPI,
        // passing these values directly to the native child WebView2 can
        // render a view at ~2/3 of the desired position and size.
        const ratio = window.devicePixelRatio || 1;
        const physical = toPhysicalWebviewRect({
          left: rect.x, top: rect.y, width: rect.width, height: rect.height,
        }, ratio);
        const key = physicalWebviewGeometryKey(physical);
        if (key === lastGeometry) return true;

        // Hide before moving: an incorrectly positioned native child must
        // never paint over the PDF or the ChatGPT dock toolbar.
        await target.hide();
        const { PhysicalPosition, PhysicalSize } = await import('@tauri-apps/api/dpi');
        await target.setPosition(new PhysicalPosition(physical.x, physical.y));
        await target.setSize(new PhysicalSize(physical.width, physical.height));

        // A CHILD Webview.position() is already relative to its parent window.
        // Do not subtract Window.innerPosition(): that mistakenly removed the
        // Windows 23px title bar and hid a correctly positioned WebView2.
        const [nativePosition, nativeSize] = await Promise.all([
          target.position(), target.size(),
        ]);
        const actual = {
          x: nativePosition.x,
          y: nativePosition.y,
          width: nativeSize.width,
          height: nativeSize.height,
        };
        if (!matchesPhysicalWebviewRect(physical, actual)) {
          throw new Error(
            `WebView2 定位校验失败：期望 ${JSON.stringify(physical)}，实际 ${JSON.stringify(actual)}，DPI=${ratio}。已隐藏错位窗口，请使用外部打开。`,
          );
        }
        lastGeometry = key;
        if (!resizingRef.current && !cancelled && childRef.current === target) await target.show();
        setDetails('');
        setChildLabel(target.label);
        setState('ready');
        return true;
      }).catch(error => {
        if (!cancelled) {
          setDetails(`原生窗口定位失败：${String(error)}`);
          setState('error');
          void target.hide().catch(() => {});
        }
        return false;
      });
      return busy;
    };
    alignRef.current = align;
    const scheduleAlign = () => {
      window.cancelAnimationFrame(scheduled);
      scheduled = window.requestAnimationFrame(() => { void align(); });
    };

    const start = async () => {
      try {
        const [{ Webview }, { getCurrentWindow }] = await Promise.all([
          import('@tauri-apps/api/webview'), import('@tauri-apps/api/window'),
        ]);
        if (cancelled || !mountRef.current) return;
        const label = `chatgpt-embedded-${crypto.randomUUID().replace(/-/g, '').slice(0, 16)}`;
        child = new Webview(getCurrentWindow(), label, {
          url: CHATGPT_URL,
          // Start as a 1px child and reveal only after physical-pixel alignment.
          x: 0, y: 0, width: 1, height: 1,
        });
        const view = child;
        childRef.current = view;
        view.once('tauri://created', () => {
          if (cancelled) { void view.hide().catch(() => {}).then(() => view.close()).catch(() => {}); return; }
          mounted = true;
          observer = new ResizeObserver(scheduleAlign);
          if (mountRef.current) {
            observer.observe(mountRef.current);
            if (mountRef.current.parentElement) observer.observe(mountRef.current.parentElement);
          }
          window.addEventListener('resize', scheduleAlign);
          window.addEventListener('scroll', scheduleAlign, true);
          viewport?.addEventListener('resize', scheduleAlign);
          viewport?.addEventListener('scroll', scheduleAlign);
          // Flex layout changes can shift x/y without a ResizeObserver event.
          // Reconcile sparingly; unchanged geometry never issues native calls.
          geometryTimer = setInterval(scheduleAlign, 250);
          // First alignment must be verified before showing the remote page.
          void align().then(aligned => {
            if (aligned && !cancelled && childRef.current === view) setChildLabel(label);
          });
        });
        view.once('tauri://error', (event) => {
          if (cancelled) return;
          setDetails(String(event.payload || 'WebView2 无法创建远程页面'));
          setState('error');
        });
      } catch (error) {
        if (!cancelled) { setDetails(String(error)); setState('error'); }
      }
    };
    void start();
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(scheduled);
      if (geometryTimer !== null) clearInterval(geometryTimer);
      observer?.disconnect();
      window.removeEventListener('resize', scheduleAlign);
      window.removeEventListener('scroll', scheduleAlign, true);
      viewport?.removeEventListener('resize', scheduleAlign);
      viewport?.removeEventListener('scroll', scheduleAlign);
      alignRef.current = null;
      if (childRef.current === child) childRef.current = null;
      // Hide first, then close: a slow/failing WebView2 close must never overlay Word.
      // Keep the remote page isolated from all Tauri local-file and SQLite capabilities.
      if (child) {
        const view = child;
        void view.hide().catch(error => console.warn('Hide ChatGPT webview on unmount:', error))
          .then(() => view.close()).catch(error => console.warn('Close ChatGPT webview on unmount:', error));
      }
    };
  }, [browserKey]);

  useEffect(() => {
    const bridge=electronBridge();
    if (!bridge) return;
    const owner=crypto.randomUUID();
    let cancelled=false, busy=false, last='';
    const align=async()=>{
      if(cancelled||busy) return;
      const rect=mountRef.current?.getBoundingClientRect();
      const geometry=rect&&!resizingRef.current&&rect.width>=40&&rect.height>=40
        ? {x:rect.left,y:rect.top,width:rect.width,height:rect.height}:null;
      const key=JSON.stringify(geometry);
      if(key===last) return;
      busy=true;
      try {
        const aligned=await bridge.chatBounds(owner,geometry);
        if(!cancelled){last=key;if(aligned&&geometry){setState('ready');setDetails('');}}
      } catch(error){if(!cancelled){setDetails(String(error));setState('error');}}
      finally{busy=false;}
    };
    let timer:ReturnType<typeof setInterval>|undefined;
    void bridge.chatOpen(owner).then(()=>{
      if(cancelled){void bridge.chatClose(owner);return;}
      void align();timer=setInterval(()=>void align(),150);
    }).catch(error=>{if(!cancelled){setDetails(String(error));setState('error');}});
    return ()=>{cancelled=true;clearInterval(timer);void bridge.chatClose(owner).catch(()=>{});};
  }, [browserKey]);

  const openExternal = async () => {
    try {
      const bridge=electronBridge();
      if(bridge) await bridge.chatExternal();
      else if (isTauri()) {
        const { openUrl } = await import('@tauri-apps/plugin-opener');
        await openUrl(CHATGPT_URL);
      } else window.open(CHATGPT_URL, '_blank', 'noopener,noreferrer');
    } catch (error) { onNotify(`无法在系统浏览器打开 ChatGPT：${String(error)}`, 'error'); }
  };

  const fillPreparedPrompt = async (prompt: string, label: string) => {
    if (!isTauri()) { setFillStatus('当前是网页预览，需通过复制提示词手动粘贴。'); return; }
    setFilling(true);
    setFillStatus('正在检查 ChatGPT 输入框…');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<string>('chatgpt_fill_prompt', { label, prompt });
      const messages: Record<string, string> = {
        filled: '提示词已自动填入。请先核对，再手动点击 ChatGPT 发送。',
        already_filled: '输入框已有相同提示词，没有重复插入。请手动确认发送。',
        occupied: '输入框已有其他文字，为避免覆盖未自动填写。可先清空后重试，或复制提示词。',
        not_ready: '输入框尚未出现；请确认已登录，页面加载后点「重试填写」。',
        unsupported: '当前 ChatGPT 输入组件暂不支持自动填写，请使用「复制提示词」。',
        login_required: 'ChatGPT 尚未完成登录；登录成功后点击「重试填写」。',
        closed: 'ChatGPT 网页已关闭，请重新打开后重试。',
        wrong_origin: '当前网页并非 chatgpt.com，为安全起见未填写。',
      };
      setFillStatus(messages[result] || '未能确认填写结果，请检查输入框并使用复制回退。');
    } catch (error) {
      setFillStatus(`自动填写不可用（${String(error)}）。请使用「复制提示词」。`);
    } finally { setFilling(false); }
  };

  // One-shot only for each user-initiated PDF analysis. Reopening the tab does not
  // silently re-insert an old prompt. No submission or response extraction occurs.
  useEffect(() => {
    if (!preparedPrompt.trim() || !preparedPromptRequestId || state !== 'ready' || !childLabel) return;
    onPromptHandled?.(preparedPromptRequestId);
    void fillPreparedPrompt(preparedPrompt, childLabel);
    // This effect is keyed to the user request and native child webview creation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preparedPromptRequestId, state, childLabel]);

  const copyPreparedPrompt = async () => {
    if (!preparedPrompt) return;
    try {
      if (isTauri()) await import('@tauri-apps/plugin-clipboard-manager').then(mod => mod.writeText(preparedPrompt));
      else await navigator.clipboard.writeText(preparedPrompt);
      onNotify('科研提示词已复制。在 ChatGPT 输入框按 Ctrl+V，核对后手动发送。');
    } catch (error) { onNotify(`复制科研提示词失败：${String(error)}`, 'error'); }
  };

  const importClipboard = async () => {
    setReading(true);
    try {
      const text = isTauri()
        ? await import('@tauri-apps/plugin-clipboard-manager').then(mod => mod.readText())
        : await navigator.clipboard.readText();
      if (!text.trim()) throw new Error('剪贴板为空。请先在 ChatGPT 页面选择内容并按 Ctrl+C。');
      if (text.length > 250_000) throw new Error('内容超过 25 万字符，请分段保存。');
      setCaptureText(text);
      const title = text.split(/\r?\n/).map(line => line.trim()).find(Boolean) || 'ChatGPT 科研记录';
      setCaptureTitle(title.slice(0, 90));
      setPaperId(papers.find(item => item.fingerprint && item.fingerprint === activeFingerprint)?.id || '');
      setPage(activePage);
    } catch (error) { onNotify(`读取剪贴板失败：${String(error)}`, 'error'); }
    finally { setReading(false); }
  };

  const quickSave = async () => {
    if (!validChatGptSource(sourceUrl)) { onNotify('请输入 ChatGPT 的 HTTPS 来源链接', 'error'); return; }
    setSaving(true);
    try {
      const now = new Date().toISOString();
      const item = papers.find(x => x.id === paperId);
      await saveClip({ id: crypto.randomUUID(), title: captureTitle, body: captureText, sourceUrl,
        itemId: paperId || null, fingerprint: item?.fingerprint || activeFingerprint, page,
        tags: '', createdAt: now, updatedAt: now });
      setCaptureText('');
      onNotify('ChatGPT 选中内容已保存到科研记录');
    } catch (error) { onNotify(`保存失败：${String(error)}`, 'error'); }
    finally { setSaving(false); }
  };

  return <section className="chat-browser-dock" aria-label="内置 ChatGPT 网页">
    <div className="chat-dock-toolbar">
      <div className="chat-dock-heading"><Globe size={17}/><strong>ChatGPT 网页</strong><span className="chat-dock-host">chatgpt.com</span></div>
      {onTogglePdf && <button className="button chat-pdf-toggle" aria-label={pdfCollapsed ? '显示 PDF 阅读区' : '隐藏 PDF 阅读区'} aria-pressed={pdfCollapsed} onClick={onTogglePdf} title={pdfCollapsed ? '重新展开左侧 PDF，恢复原分屏比例' : '隐藏左侧 PDF，让 ChatGPT 使用完整工作区'}>{pdfCollapsed ? <PanelLeftOpen size={15}/> : <PanelLeftClose size={15}/>} {pdfCollapsed ? '显示 PDF' : '隐藏 PDF'}</button>}
      <button className="icon-button" title="重新打开 ChatGPT 网页" onClick={() => { setState('starting'); setBrowserKey(v => v + 1); }}><RefreshCw size={16}/></button>
      <button className="button" onClick={() => void openExternal()} title="登录受阻时使用系统浏览器"><ExternalLink size={15}/> 外部打开</button>
      <button className="icon-button" aria-label="关闭 ChatGPT 侧栏" title="关闭 ChatGPT 侧栏，恢复左侧完整宽度" onClick={onClose}><X size={18}/></button>
    </div>
    <div className="chat-native-viewport" ref={mountRef}>
      {state !== 'ready' && <div className="chat-browser-placeholder">
        {state === 'starting' ? <LoaderCircle className="spin" size={24}/> : <Globe size={28}/>}
        <strong>{state === 'starting' ? (electronBridge() ? '正在加载 Chromium 分屏…' : '正在加载 Windows WebView2…') : '内置网页不可用'}</strong>
        <p>{state === 'starting' ? '网页加载后可直接登录已有 ChatGPT 账号。' : details || '浏览器预览模式或当前环境不支持嵌入式网页。'}</p>
        {state !== 'starting' && <button className="button primary" onClick={() => void openExternal()}><ExternalLink size={15}/> 在浏览器打开 ChatGPT</button>}
      </div>}
    </div>
    {activePaperName && <section className={`chat-prompt-panel ${promptCollapsed ? 'chat-prompt-collapsed' : ''}`} aria-label="当前 PDF 科研提示词">
      <div className="chat-prompt-heading">
        <div className="chat-prompt-title">
          <strong>{preparedPrompt ? '来自当前 PDF 的科研分析提示词（不会自动发送）' : '当前论文 · 尚无待发送提示词'}</strong>
          <span title={activePaperName}>当前论文：{activePaperName}</span>
        </div>
        <button type="button" className="button chat-prompt-toggle" aria-expanded={!promptCollapsed}
          onClick={() => setPromptCollapsed(value => !value)} title={promptCollapsed ? '展开底部科研提示词窗口' : '隐藏底部科研提示词窗口'}>
          {promptCollapsed ? <ChevronUp size={14}/> : <ChevronDown size={14}/>}
          {promptCollapsed ? '展开提示词' : '隐藏提示词'}
        </button>
      </div>
      {!promptCollapsed && (preparedPrompt ? <>
        <textarea readOnly aria-label="科研分析提示词" value={preparedPrompt} rows={3}/>
        <div className="chat-prompt-actions">
          {isTauri() && <button className="button" disabled={filling || state !== 'ready' || !childLabel} onClick={() => { if (childLabel) void fillPreparedPrompt(preparedPrompt, childLabel); }}>{filling ? '自动填写中…' : '重试自动填写'}</button>}
          <button className="button primary" onClick={() => void copyPreparedPrompt()}><ClipboardPaste size={14}/> 复制提示词 · Ctrl+V 粘贴</button>
        </div>
        {fillStatus && <small className="chat-fill-status" role="status">{fillStatus}</small>}
      </> : <small className="chat-prompt-empty">此 PDF 暂无科研提示词。请在左侧 PDF 划词并选择科研分析操作。</small>)}
    </section>}
    {captureText && <div className="chat-inline-save">
      <div className="chat-inline-title"><strong>保存到科研记录</strong><button title="取消" className="icon-button" onClick={() => setCaptureText('')}><X size={16}/></button></div>
      <input aria-label="科研记录标题" maxLength={240} value={captureTitle} onChange={e => setCaptureTitle(e.target.value)} placeholder="记录标题"/>
      <textarea aria-label="待保存文本" value={captureText} onChange={e => setCaptureText(e.target.value)} rows={3}/><div className="chat-inline-fields">
      <select aria-label="关联论文" value={paperId} onChange={e => setPaperId(e.target.value)}><option value="">不关联文献</option>{papers.map(x => <option key={x.id} value={x.id}>{x.title}</option>)}</select>
      <input aria-label="关联页码" type="number" min={1} placeholder="页码" value={page ?? ''} onChange={e => setPage(e.target.value ? Number(e.target.value) : null)}/></div>
      <input aria-label="ChatGPT 来源链接" value={sourceUrl} onChange={e => setSourceUrl(e.target.value)} placeholder="https://chatgpt.com/c/..."/>
      <div className="chat-inline-actions"><button className="button" onClick={() => onCaptureText(captureText)}>详细编辑</button><button className="button primary" disabled={saving || !captureText.trim() || !captureTitle.trim()} onClick={() => void quickSave()}><Save size={14}/>{saving ? '保存中…' : '确认保存'}</button></div>
    </div>}
    <div className="chat-capture-bar">
      <Info size={16}/>
      <span>在上方网页中选中回答并按 Ctrl+C，再点击保存。不会自动读取聊天记录或登录信息。</span>
      <button className="button primary" disabled={reading} onClick={() => void importClipboard()}><ClipboardPaste size={16}/>{reading ? '读取中…' : '保存选中内容'}</button>
    </div>
  </section>;
}
