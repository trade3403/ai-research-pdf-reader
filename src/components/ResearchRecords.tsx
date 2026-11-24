import { useCallback, useEffect, useMemo, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { BookOpen, ClipboardPaste, Download, FileText, Link2, NotebookPen, Plus, Save, Search, Tag, Trash2, Code2, Eye, Columns2, Languages, FilePenLine } from 'lucide-react';
import { loadLibrary } from '../utils/libraryStore';
import type { LiteratureItem } from '../utils/libraryStore';
import { clipsToMarkdown, deleteClip, listClips, saveClip } from '../utils/researchClips';
import type { ResearchClip } from '../utils/researchClips';
import ResearchMarkdown from './ResearchMarkdown';

export type ClipDraftSeed = { body: string; title?: string; itemId?: string | null; fingerprint?: string | null; page?: number | null };

type Props = {
  initialDraft: ClipDraftSeed | null;
  onDraftConsumed: () => void;
  onOpenPdf: (path: string, page?: number | null) => Promise<void>;
  onNotify: (text: string, tone?: 'error' | 'info') => void;
  onTranslate: (source:string,label:string) => void;
  onInsertToWord: (body:string,title:string,source:string)=>void;
};

function makeDraft(seed?: ClipDraftSeed | null): ResearchClip {
  const now = new Date().toISOString();
  const body = seed?.body || '';
  const preview = body.split(/\r?\n/).map(x => x.trim()).find(Boolean) || '';
  return {
    id: crypto.randomUUID(), title: seed?.title || preview.slice(0, 90),
    body, sourceUrl: 'https://chatgpt.com/', itemId: seed?.itemId ?? null,
    fingerprint: seed?.fingerprint ?? null, page: seed?.page ?? null, tags: '',
    createdAt: now, updatedAt: now,
  };
}

export default function ResearchRecords({ initialDraft, onDraftConsumed, onOpenPdf, onNotify, onTranslate, onInsertToWord }: Props) {
  const [clips, setClips] = useState<ResearchClip[]>([]);
  const [papers, setPapers] = useState<LiteratureItem[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<ResearchClip>(() => makeDraft());
  const [query, setQuery] = useState('');
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [showEditor, setShowEditor] = useState(false);
  const [bodyView, setBodyView] = useState<'preview' | 'edit' | 'split'>('preview');

  const reload = useCallback(async () => {
    try {
      const [items, library] = await Promise.all([listClips(), loadLibrary()]);
      setClips(items);
      setPapers(library.items);
    } catch (error) { onNotify(`科研记录读取失败：${String(error)}`, 'error'); }
  }, [onNotify]);
  useEffect(() => { void reload(); }, [reload]);

  useEffect(() => {
    if (initialDraft === null) return;
    setDraft(makeDraft(initialDraft));
    setSelectedId(null);
    setDirty(true);
    setShowEditor(true);
    setBodyView('preview');
    onDraftConsumed();
  }, [initialDraft, onDraftConsumed]);

  const filtered = useMemo(() => {
    const term = query.trim().toLocaleLowerCase();
    return clips.filter(clip => !term || [clip.title, clip.body, clip.tags, clip.sourceUrl].some(x => x.toLocaleLowerCase().includes(term)));
  }, [clips, query]);
  const paperById = useMemo(() => new Map(papers.map(item => [item.id, item])), [papers]);
  const update = (patch: Partial<ResearchClip>) => { setDraft(current => ({ ...current, ...patch })); setDirty(true); };

  const choose = (clip: ResearchClip) => {
    if (dirty && !window.confirm('当前未保存的更改会丢失，继续吗？')) return;
    setSelectedId(clip.id);
    setDraft({ ...clip });
    setShowEditor(true);
    setBodyView('preview');
    setDirty(false);
  };
  const makeNew = () => {
    if (dirty && !window.confirm('放弃当前未保存的更改？')) return;
    setSelectedId(null);
    setDraft(makeDraft());
    setDirty(false);
    setShowEditor(true);
    setBodyView('edit');
  };

  const save = async () => {
    setSaving(true);
    try {
      const chosenPaper = draft.itemId ? paperById.get(draft.itemId) : null;
      const clean = await saveClip({ ...draft, fingerprint: chosenPaper?.fingerprint ?? draft.fingerprint, updatedAt: new Date().toISOString() });
      await reload();
      setDraft(clean);
      setSelectedId(clean.id);
      setDirty(false);
      setBodyView('preview');
      onNotify('科研记录已保存到本地数据库');
    } catch (error) { onNotify(`保存失败：${String(error)}`, 'error'); }
    finally { setSaving(false); }
  };
  const remove = async () => {
    if (!selectedId || !window.confirm('确认删除这条科研记录？此操作不能撤销。')) return;
    try { await deleteClip(selectedId); setSelectedId(null); setShowEditor(false); setDirty(false); await reload(); onNotify('科研记录已删除'); }
    catch (error) { onNotify(`删除失败：${String(error)}`, 'error'); }
  };

  const readClipboard = async () => {
    try {
      const text = isTauri()
        ? await import('@tauri-apps/plugin-clipboard-manager').then(m => m.readText())
        : await navigator.clipboard.readText();
      if (!text.trim()) throw new Error('剪贴板为空');
      if (text.length > 250_000) throw new Error('内容超过 25 万字符');
      if (!showEditor) { setDraft(makeDraft({ body: text })); setShowEditor(true); setSelectedId(null); }
      else update({ body: draft.body ? `${draft.body}\n\n${text}` : text });
      setDirty(true);
      setBodyView('preview');
    } catch (error) { onNotify(`读取剪贴板失败：${String(error)}`, 'error'); }
  };

  const exportRecords = async () => {
    try {
      if (!clips.length) throw new Error('没有可以导出的记录');
      const content = clipsToMarkdown(filtered);
      if (isTauri()) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const path = await save({ defaultPath: 'ChatGPT-科研记录.md', filters: [{ name: 'Markdown', extensions: ['md'] }] });
        if (!path) return;
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('write_markdown', { path, content });
      } else {
        const url = URL.createObjectURL(new Blob([content], { type: 'text/markdown;charset=utf-8' }));
        const anchor = document.createElement('a'); anchor.href = url; anchor.download = 'ChatGPT-科研记录.md'; anchor.click();
        window.setTimeout(() => URL.revokeObjectURL(url), 1000);
      }
      onNotify('科研记录 Markdown 已导出');
    } catch (error) { onNotify(`导出失败：${String(error)}`, 'error'); }
  };

  return <div className="records-workspace">
    <aside className="records-list-pane">
      <div className="records-list-header"><div><NotebookPen size={19}/><strong>ChatGPT 科研记录</strong></div><button className="button primary" onClick={makeNew}><Plus size={15}/> 新建</button></div>
      <div className="records-search"><Search size={16}/><input aria-label="搜索科研记录" placeholder="搜索标题、正文、标签…" value={query} onChange={e => setQuery(e.target.value)}/></div>
      <div className="records-actions"><span>共 {filtered.length} 条</span><button onClick={() => void exportRecords()}><Download size={15}/> 导出 Markdown</button></div>
      <div className="records-list">
        {filtered.map(clip => <button key={clip.id} className={`records-item ${selectedId === clip.id ? 'active' : ''}`} onClick={() => choose(clip)}>
          <strong>{clip.title}</strong>
          <span>{clip.body.replace(/\s+/g, ' ').slice(0, 115)}</span>
          <small>{new Date(clip.updatedAt).toLocaleString()} {clip.itemId && '· 已关联论文'}</small>
        </button>)}
        {!filtered.length && <div className="records-blank"><NotebookPen size={26}/><p>暂无科研记录</p><span>在 ChatGPT 中复制回答，再从内置工具栏保存到这里。</span></div>}
      </div>
    </aside>
    <main className="records-editor-pane">
      {showEditor ? <>
        <div className="records-editor-head"><div><strong>{selectedId ? '编辑科研记录' : '新建科研记录'}</strong><small>本地 SQLite · 用户主动保存 · 不自动读取 ChatGPT 页面</small></div><div className="records-editor-actions"><button className="button" title="将当前科研记录内容插入 Word" disabled={!draft.body.trim()} onClick={()=>onInsertToWord(draft.body,draft.title,draft.sourceUrl)}><FilePenLine size={15}/> 插入 Word</button><button className="button" title="中英对照科研记录" disabled={!draft.body.trim()} onClick={()=>onTranslate(draft.body,draft.title || 'ChatGPT 科研记录')}><Languages size={15}/> 翻译 / 中英对照</button><button className="button" onClick={() => void readClipboard()}><ClipboardPaste size={15}/> 粘贴剪贴板</button><button className="button primary" disabled={saving || !draft.body.trim()} onClick={() => void save()}><Save size={15}/>{saving ? '保存中…' : dirty ? '保存更改' : '保存'}</button>{selectedId && <button className="icon-button danger" title="删除记录" onClick={() => void remove()}><Trash2 size={17}/></button>}</div></div>
        <div className="records-form">
          <label>记录标题<input maxLength={240} value={draft.title} onChange={e => update({ title:e.target.value })} placeholder="研究问题或结论标题"/></label>
          <div className="records-content-field">
            <div className="records-content-toolbar">
              <strong>ChatGPT 内容 · Markdown / LaTeX</strong>
              <div className="records-view-modes" role="group" aria-label="科研记录正文显示方式">
                <button type="button" className={bodyView === 'preview' ? 'active' : ''} aria-pressed={bodyView === 'preview'} onClick={() => setBodyView('preview')}><Eye size={14}/> 排版预览</button>
                <button type="button" className={bodyView === 'edit' ? 'active' : ''} aria-pressed={bodyView === 'edit'} onClick={() => setBodyView('edit')}><Code2 size={14}/> 编辑原文</button>
                <button type="button" className={bodyView === 'split' ? 'active' : ''} aria-pressed={bodyView === 'split'} onClick={() => setBodyView('split')}><Columns2 size={14}/> 分栏</button>
              </div>
            </div>
            <div className={`records-content-layout ${bodyView === 'split' ? 'is-split' : ''}`}>
              {(bodyView === 'edit' || bodyView === 'split') && <textarea
                aria-label="科研记录 Markdown 和 LaTeX 原文"
                className="records-body"
                value={draft.body}
                onChange={e => update({ body: e.target.value })}
                placeholder="粘贴 ChatGPT 回答；支持 $...$、$$...$$、\(...\) 和 \[...\] 数学公式"
                spellCheck={false}
              />}
              {(bodyView === 'preview' || bodyView === 'split') && <div className="records-rich-preview" aria-label="科研记录排版预览">
                <ResearchMarkdown source={draft.body} />
              </div>}
            </div>
            <small className="records-content-help">公式在本地排版显示；原始 Markdown / LaTeX 会原样保存，切换模式不影响记录内容。</small>
          </div>
          <div className="records-form-grid">
            <label><span><Link2 size={14}/> 来源链接</span><input type="url" value={draft.sourceUrl} onChange={e => update({sourceUrl:e.target.value})} placeholder="https://chatgpt.com/c/..."/></label>
            <label><span><Tag size={14}/> 标签</span><input value={draft.tags} onChange={e => update({tags:e.target.value})} placeholder="稳定性、算法、实验"/></label>
          </div>
          <div className="records-form-grid">
            <label><span><BookOpen size={14}/> 关联文献（可选）</span><select value={draft.itemId || ''} onChange={e => {const item=paperById.get(e.target.value); update({itemId:e.target.value || null, fingerprint:item?.fingerprint || null});}}><option value="">不关联文献</option>{papers.map(item=><option key={item.id} value={item.id}>{item.title}</option>)}</select></label>
            <label><span><FileText size={14}/> PDF 页码（可选）</span><input type="number" min={1} value={draft.page ?? ''} onChange={e => update({page:e.target.value ? Number(e.target.value) : null})} /></label>
          </div>
          {draft.itemId && paperById.get(draft.itemId)?.path && <button className="button" onClick={() => { const path = paperById.get(draft.itemId || '')?.path; if (path) void onOpenPdf(path, draft.page); }}><BookOpen size={15}/> 打开关联论文</button>}
          <p className="records-footnote">ChatGPT 网页不会直接访问本地文件或数据库。仅当你点击保存时才会把上述内容写入本机；请勿保存密码或敏感凭据。</p>
        </div>
      </> : <div className="records-welcome"><NotebookPen size={44}/><h2>统一管理科研讨论</h2><p>在 ChatGPT 页面中复制需要保留的回答，再进入本页保存到文献库。</p><button className="button primary" onClick={makeNew}><Plus size={17}/> 创建科研记录</button></div>}
    </main>
  </div>;
}
