import { isTauri } from '@tauri-apps/api/core';
import { getDatabase } from './readerStore';

/** Notes are explicitly captured by the user; never read ChatGPT's DOM or session. */
export type ResearchClip = {
  id: string;
  title: string;
  body: string;
  sourceUrl: string;
  itemId: string | null;
  fingerprint: string | null;
  page: number | null;
  tags: string;
  createdAt: string;
  updatedAt: string;
};

const CLIPS_STORAGE_KEY = 'research-reader-v04-clips';
const MAX_TITLE = 240;
const MAX_BODY = 250_000;
const MAX_TAGS = 400;
let initializing: Promise<void> | null = null;

type ClipRow = {
  id: string;
  title: string;
  body: string;
  source_url: string;
  item_id: string | null;
  fingerprint: string | null;
  page: number | null;
  tags: string;
  created_at: string;
  updated_at: string;
};

function fromRow(row: ClipRow): ResearchClip {
  return {
    id: row.id, title: row.title, body: row.body, sourceUrl: row.source_url,
    itemId: row.item_id, fingerprint: row.fingerprint, page: row.page,
    tags: row.tags, createdAt: row.created_at, updatedAt: row.updated_at,
  };
}

function browserLoad(): ResearchClip[] {
  try { const parsed = JSON.parse(localStorage.getItem(CLIPS_STORAGE_KEY) || '[]'); return Array.isArray(parsed) ? parsed as ResearchClip[] : []; }
  catch { return []; }
}

function browserWrite(clips: ResearchClip[]): void {
  localStorage.setItem(CLIPS_STORAGE_KEY, JSON.stringify(clips));
}

/** Never accept javascript:, file:, or an unrelated site's URL as a ChatGPT source. */
export function validChatGptSource(url: string): boolean {
  try {
    const parsed = new URL(url.trim());
    return parsed.protocol === 'https:' && ['chatgpt.com', 'chat.openai.com'].includes(parsed.hostname)
      && !parsed.username && !parsed.password;
  } catch { return false; }
}

export function normalizeClip(clip: ResearchClip): ResearchClip {
  const title = clip.title.trim();
  const body = clip.body.trim();
  if (!title || title.length > MAX_TITLE) throw new Error(`标题必须在 1～${MAX_TITLE} 字符之间`);
  if (!body || body.length > MAX_BODY) throw new Error(`科研记录必须在 1～${MAX_BODY} 字符之间`);
  if (!validChatGptSource(clip.sourceUrl)) throw new Error('来源链接必须为 https://chatgpt.com 或 https://chat.openai.com');
  if (clip.tags.length > MAX_TAGS) throw new Error('标签过长');
  if (clip.page !== null && (!Number.isSafeInteger(clip.page) || clip.page < 1 || clip.page > 1_000_000)) throw new Error('页码无效');
  return { ...clip, title, body, sourceUrl: clip.sourceUrl.trim(), tags: clip.tags.trim() };
}

async function initClips(): Promise<void> {
  if (!isTauri()) return;
  if (!initializing) {
    initializing = (async () => {
      const db = await getDatabase();
      await db.execute(`CREATE TABLE IF NOT EXISTS research_clips (
        id TEXT PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL,
        source_url TEXT NOT NULL, item_id TEXT, fingerprint TEXT, page INTEGER,
        tags TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, updated_at TEXT NOT NULL
      )`);
      await db.execute('CREATE INDEX IF NOT EXISTS idx_research_clips_recent ON research_clips(updated_at DESC)');
      await db.execute('CREATE INDEX IF NOT EXISTS idx_research_clips_item ON research_clips(item_id)');
    })().catch(error => { initializing = null; throw error; });
  }
  return initializing;
}

export async function listClips(): Promise<ResearchClip[]> {
  if (!isTauri()) return browserLoad().sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  await initClips();
  const db = await getDatabase();
  const rows = await db.select<ClipRow[]>('SELECT * FROM research_clips ORDER BY updated_at DESC LIMIT 10000');
  return rows.map(fromRow);
}

/** Upsert keeps the original creation timestamp when editing a saved record. */
export async function saveClip(clip: ResearchClip): Promise<ResearchClip> {
  const safe = normalizeClip(clip);
  if (!isTauri()) {
    browserWrite([safe, ...browserLoad().filter(c => c.id !== safe.id)]);
    return safe;
  }
  await initClips();
  const db = await getDatabase();
  await db.execute(`INSERT INTO research_clips
     (id,title,body,source_url,item_id,fingerprint,page,tags,created_at,updated_at)
     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
     ON CONFLICT(id) DO UPDATE SET
       title=excluded.title,body=excluded.body,source_url=excluded.source_url,
       item_id=excluded.item_id,fingerprint=excluded.fingerprint,
       page=excluded.page,tags=excluded.tags,updated_at=excluded.updated_at`,
  [safe.id, safe.title, safe.body, safe.sourceUrl, safe.itemId, safe.fingerprint,
    safe.page, safe.tags, safe.createdAt, safe.updatedAt]);
  return safe;
}

export async function deleteClip(id: string): Promise<void> {
  if (!isTauri()) { browserWrite(browserLoad().filter(x => x.id !== id)); return; }
  await initClips();
  const db = await getDatabase();
  await db.execute('DELETE FROM research_clips WHERE id=$1', [id]);
}

export function clipsToMarkdown(clips: ResearchClip[]): string {
  const lines = ['# AI Research Studio — ChatGPT 科研记录', ''];
  for (const c of clips) {
    lines.push(`## ${c.title.replace(/[\r\n#]/g, ' ')}`, '');
    lines.push(`- 保存时间：${c.createdAt}`);
    lines.push(`- 来源：[ChatGPT](${validChatGptSource(c.sourceUrl) ? c.sourceUrl : 'https://chatgpt.com/'})`);
    if (c.itemId) lines.push(`- 关联文献 ID：\`${c.itemId}\``);
    if (c.page) lines.push(`- PDF 页码：${c.page}`);
    if (c.tags) lines.push(`- 标签：${c.tags.replace(/[\r\n]/g, ' ')}`);
    lines.push('', c.body, '', '---', '');
  }
  return lines.join('\n');
}
