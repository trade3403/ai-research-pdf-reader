import { useMemo, useState } from 'react';
import { ChevronRight, Download, Highlighter, MessageSquareText, Trash2, X } from 'lucide-react';
import { COLOR_HEX } from '../utils/annotations';
import type { Annotation, AnnotationColor } from '../utils/annotations';

type Props = {
  title: string;
  annotations: Annotation[];
  selectedId: string | null;
  onSelect: (annotation: Annotation) => void;
  onUpdateNote: (annotation: Annotation, note: string) => void;
  onDelete: (annotation: Annotation) => void;
  onExport: () => void;
  onClose: () => void;
};

export default function AnnotationPanel({
  title, annotations, selectedId, onSelect, onUpdateNote, onDelete, onExport, onClose,
}: Props) {
  const [filter, setFilter] = useState<'all' | AnnotationColor>('all');
  const sorted = useMemo(() => [...annotations]
    .filter((ann) => filter === 'all' || ann.color === filter)
    .sort((a, b) => a.page - b.page || a.createdAt.localeCompare(b.createdAt)), [annotations, filter]);
  const selected = annotations.find((ann) => ann.id === selectedId) || null;

  return (
    <aside className="annotation-panel" aria-label="论文批注与笔记">
      <div className="annotation-panel-header">
        <div><strong>论文批注</strong><small>{annotations.length} 条 · 本地保存</small></div>
        <button className="icon-button" title="关闭批注面板" onClick={onClose}><X size={18} /></button>
      </div>
      <div className="annotation-panel-doc" title={title}>{title}</div>
      <div className="annotation-panel-tools">
        <select value={filter} onChange={(event) => setFilter(event.target.value as typeof filter)} aria-label="批注颜色筛选">
          <option value="all">全部颜色</option>
          <option value="yellow">黄色</option>
          <option value="green">绿色</option>
          <option value="blue">蓝色</option>
          <option value="pink">粉色</option>
        </select>
        <button className="button export-button" onClick={onExport} title="导出全部批注为 Markdown"><Download size={15} />导出 MD</button>
      </div>
      <div className="annotation-list">
        {sorted.length === 0 && <div className="annotation-empty"><Highlighter size={29} />
          <p>暂无批注</p><span>在 PDF 页面中用鼠标选中文字，选择高亮颜色，即可创建批注。</span></div>}
        {sorted.map((ann) => <button key={ann.id}
          className={`annotation-card ${selectedId === ann.id ? 'active' : ''}`}
          onClick={() => onSelect(ann)}>
          <span className="annotation-card-head"><span className="annotation-dot" style={{ background: COLOR_HEX[ann.color] }} />
            第 {ann.page} 页 · {ann.kind === 'highlight' ? '高亮' : ann.kind === 'underline' ? '下划线' : '删除线'}
            <ChevronRight size={14}/></span>
          <span className="annotation-quote">{ann.quote}</span>
          {ann.note && <span className="annotation-note-excerpt"><MessageSquareText size={12}/>{ann.note}</span>}
        </button>)}
      </div>
      {selected && <div className="annotation-editor">
        <div className="annotation-editor-head"><strong>第 {selected.page} 页笔记</strong>
          <button className="icon-button" title="删除此条批注" onClick={() => onDelete(selected)}><Trash2 size={16}/></button>
        </div>
        <textarea key={selected.id} value={selected.note} rows={5}
          placeholder="记录思考、疑问、研究方法、可复现实验要点…"
          onChange={(e) => onUpdateNote(selected, e.target.value)}
          aria-label="编辑当前批注笔记" />
        <div className="annotation-editor-hint">自动保存至本机 SQLite · 不修改原 PDF</div>
      </div>}
    </aside>
  );
}
