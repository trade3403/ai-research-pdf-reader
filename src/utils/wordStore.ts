import { electronBridge } from './electronBridge';
import { isTauri } from '@tauri-apps/api/core';
import type { JSONContent } from '@tiptap/core';
import { getDatabase } from './readerStore';
import type { CitationStyle } from './wordAcademic';

/** Native DOCX files are not a safe interchange format for autosave. We keep
 * editable ProseMirror JSON in an additive SQLite table and export DOCX on demand. */
export type Manuscript = {
  id: string;
  title: string;
  content: JSONContent;
  sourcePath: string | null;
  pageSize: 'A4' | 'Letter';
  marginMm: number;
  citationStyle: CitationStyle;
  lineSpacing: number;
  headerText: string;
  footerText: string;
  createdAt: string;
  updatedAt: string;
};

type ManuscriptRow = {
  id: string; title: string; content_json: string; source_path: string | null;
  page_size: string; margin_mm: number; citation_style?:string; line_spacing?:number; header_text?:string; footer_text?:string; created_at: string; updated_at: string;
};
const LOCAL_KEY = 'research-reader-word-v05-drafts';
const MAX_JSON = 8_000_000;
let initializing: Promise<void> | null = null;

export function makeManuscript(title = '未命名科研论文'): Manuscript {
  const now = new Date().toISOString();
  return { id: crypto.randomUUID(), title, sourcePath: null, pageSize: 'A4', marginMm: 25, citationStyle:'numeric', lineSpacing:1.5, headerText:'', footerText:'',
    content: { type: 'doc', content: [{ type: 'heading', attrs: { level: 1 }, content: [{type:'text', text:title}] }, {type:'paragraph'}] },
    createdAt: now, updatedAt: now };
}
function parseRow(r: ManuscriptRow): Manuscript {
  let content: JSONContent;
  try { content = JSON.parse(r.content_json) as JSONContent; }
  catch { throw new Error(`Word 草稿 ${r.id} 的 JSON 数据损坏，请从备份恢复`); }
  return {id:r.id, title:r.title, content, sourcePath:r.source_path,
    pageSize:r.page_size === 'Letter' ? 'Letter' : 'A4', marginMm:r.margin_mm,
    citationStyle:r.citation_style==='author-year'?'author-year':'numeric',
    lineSpacing:[1,1.15,1.5,2].includes(Number(r.line_spacing))?Number(r.line_spacing):1.5,
    headerText:r.header_text||'',footerText:r.footer_text||'',
    createdAt:r.created_at, updatedAt:r.updated_at};
}
function browserList(): Manuscript[] {
  try { const data: unknown = JSON.parse(localStorage.getItem(LOCAL_KEY) || '[]'); return Array.isArray(data) ? (data as Manuscript[]).map(d=>({...d,citationStyle:d.citationStyle||'numeric',lineSpacing:d.lineSpacing||1.5,headerText:d.headerText||'',footerText:d.footerText||''})) : []; }
  catch { return []; }
}
function check(doc: Manuscript): string {
  if (!doc.id || !doc.title.trim() || doc.title.length > 240) throw new Error('文档标题长度不合法');
  if (![1,1.15,1.5,2].includes(doc.lineSpacing||1.5)) throw new Error('行距无效');
  if ((doc.headerText||'').length>150||(doc.footerText||'').length>150) throw new Error('页眉页脚过长');
  if (!(doc.marginMm >= 10 && doc.marginMm <= 45)) throw new Error('页边距应在 10–45 mm 之间');
  const text = JSON.stringify(doc.content);
  if (text.length > MAX_JSON) throw new Error('草稿已超过 8 MB，请减少嵌入图片大小');
  return text;
}
async function init(): Promise<void> {
  if (!isTauri()) return;
  if (!initializing) initializing = (async () => {
    const db = await getDatabase();
    await db.execute(`CREATE TABLE IF NOT EXISTS word_manuscripts (
      id TEXT PRIMARY KEY, title TEXT NOT NULL, content_json TEXT NOT NULL,
      source_path TEXT, page_size TEXT NOT NULL DEFAULT 'A4',
      margin_mm REAL NOT NULL DEFAULT 25, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
    )`);
    // Additive migration: old V0.5 drafts and their content remain intact.
    const columns=await db.select<Array<{name:string}>>('PRAGMA table_info(word_manuscripts)');
    for (const [name,definition] of [
      ['citation_style',"TEXT NOT NULL DEFAULT 'numeric'"],
      ['line_spacing','REAL NOT NULL DEFAULT 1.5'],
      ['header_text',"TEXT NOT NULL DEFAULT ''"],
      ['footer_text',"TEXT NOT NULL DEFAULT ''"],
    ]) {
      if(!columns.some(column=>column.name===name))
        await db.execute(`ALTER TABLE word_manuscripts ADD COLUMN ${name} ${definition}`);
    }
    await db.execute('CREATE INDEX IF NOT EXISTS idx_word_manuscripts_recent ON word_manuscripts(updated_at DESC)');
  })().catch(e => { initializing = null; throw e; });
  return initializing;
}
let migrated:Promise<unknown>|null=null;
async function nativeWord<T>(op:string,v?:unknown):Promise<T>{
 const b=electronBridge();if(!b)throw Error('Native word store unavailable');
 if(!migrated)migrated=b.research('word.import',browserList()).catch(e=>{migrated=null;throw e;});
 await migrated;return b.research(op,v) as Promise<T>;
}
export async function listManuscripts(): Promise<Manuscript[]> {
  if(electronBridge())return nativeWord<Manuscript[]>('word.list');
  if (!isTauri()) return browserList().sort((a,b) => b.updatedAt.localeCompare(a.updatedAt));
  await init(); const db=await getDatabase();
  const rows=await db.select<ManuscriptRow[]>('SELECT * FROM word_manuscripts ORDER BY updated_at DESC LIMIT 400');
  return rows.map(parseRow);
}
export async function saveManuscript(doc: Manuscript): Promise<void> {
  const content=check(doc);
  if(electronBridge()){await nativeWord('word.save',doc);return;}
  if (!isTauri()) {
    localStorage.setItem(LOCAL_KEY,JSON.stringify([doc,...browserList().filter(d=>d.id!==doc.id)].slice(0,200)));
    return;
  }
  await init();const db=await getDatabase();
  await db.execute(`INSERT INTO word_manuscripts (id,title,content_json,source_path,page_size,margin_mm,created_at,updated_at,citation_style,line_spacing,header_text,footer_text)
    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
    ON CONFLICT(id) DO UPDATE SET title=excluded.title, content_json=excluded.content_json,
      source_path=excluded.source_path, page_size=excluded.page_size,
      margin_mm=excluded.margin_mm, updated_at=excluded.updated_at, citation_style=excluded.citation_style,
      line_spacing=excluded.line_spacing,header_text=excluded.header_text,footer_text=excluded.footer_text`,
    [doc.id,doc.title,content,doc.sourcePath,doc.pageSize,doc.marginMm,doc.createdAt,doc.updatedAt,doc.citationStyle||'numeric',doc.lineSpacing||1.5,doc.headerText||'',doc.footerText||'']);
}
export async function deleteManuscript(id: string): Promise<void> {
  if(electronBridge()){await nativeWord('word.delete',id);return;}
  if(!isTauri()) {localStorage.setItem(LOCAL_KEY,JSON.stringify(browserList().filter(d=>d.id!==id)));return;}
  await init();const db=await getDatabase();await db.execute('DELETE FROM word_manuscripts WHERE id=$1',[id]);
}
