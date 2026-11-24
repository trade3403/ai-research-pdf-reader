import { electronBridge } from './electronBridge';
import { isTauri } from '@tauri-apps/api/core';
import { getDatabase } from './readerStore';

export type LiteratureItem = {
  id: string; fingerprint: string | null; sha256: string | null; path: string | null;
  filename: string; fileSize: number; title: string; authors: string; year: number | null;
  journal: string; doi: string; abstract: string; starred: boolean;
  addedAt: string; updatedAt: string; indexStatus: 'none' | 'partial' | 'complete';
  tags: string[]; collectionIds: string[];
};
export type Collection = { id: string; name: string; parentId: string | null };
export type LibrarySnapshot = { items: LiteratureItem[]; collections: Collection[] };
export type ImportStatus = 'inserted' | 'duplicate-hash' | 'duplicate-path';
const NOW = () => new Date().toISOString();
const BROWSER_KEY = 'research-reader-v03-library';
let initialized: Promise<void> | null = null;

export async function initLibrary(): Promise<void> {
  if (!isTauri()) return;
  if (!initialized) initialized = (async () => {
    const db = await getDatabase();
    await db.execute(`CREATE TABLE IF NOT EXISTS library_items (
      id TEXT PRIMARY KEY, fingerprint TEXT, sha256 TEXT, path TEXT UNIQUE,
      filename TEXT NOT NULL, file_size INTEGER NOT NULL DEFAULT 0,
      title TEXT NOT NULL, authors TEXT NOT NULL DEFAULT '', year INTEGER,
      journal TEXT NOT NULL DEFAULT '', doi TEXT NOT NULL DEFAULT '', abstract TEXT NOT NULL DEFAULT '',
      starred INTEGER NOT NULL DEFAULT 0, index_status TEXT NOT NULL DEFAULT 'none',
      added_at TEXT NOT NULL, updated_at TEXT NOT NULL)`);
    await db.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_library_sha ON library_items(sha256) WHERE sha256 IS NOT NULL");
    await db.execute('CREATE INDEX IF NOT EXISTS idx_library_doi ON library_items(doi)');
    await db.execute('CREATE INDEX IF NOT EXISTS idx_library_title ON library_items(title)');
    await db.execute(`CREATE TABLE IF NOT EXISTS library_collections (id TEXT PRIMARY KEY, name TEXT NOT NULL, parent_id TEXT)`);
    await db.execute(`CREATE TABLE IF NOT EXISTS library_memberships (
      item_id TEXT NOT NULL, collection_id TEXT NOT NULL,
      PRIMARY KEY(item_id, collection_id))`);
    await db.execute(`CREATE TABLE IF NOT EXISTS library_tags (
      item_id TEXT NOT NULL, tag TEXT NOT NULL, PRIMARY KEY(item_id, tag))`);
    await db.execute(`CREATE TABLE IF NOT EXISTS library_pages (
      item_id TEXT NOT NULL, page INTEGER NOT NULL, content TEXT NOT NULL,
      PRIMARY KEY(item_id, page))`);
    await db.execute('CREATE INDEX IF NOT EXISTS idx_library_pages_item ON library_pages(item_id)');
    // FTS is optional; metadata search continues working on SQLite builds without FTS5.
    try { await db.execute("CREATE VIRTUAL TABLE IF NOT EXISTS library_fts USING fts5(item_id UNINDEXED, page UNINDEXED, content, tokenize='unicode61')"); }
    catch (error) { console.warn('FTS5 unavailable; searching with SQLite LIKE', error); }
  })().catch((error) => { initialized = null; throw error; });
  return initialized;
}

type ItemRow = {
  id: string; fingerprint: string | null; sha256: string | null; path: string | null;
  filename: string; file_size: number; title: string; authors: string; year: number | null;
  journal: string; doi: string; abstract: string; starred: number;
  added_at: string; updated_at: string; index_status: string;
};
function fromRow(r: ItemRow): LiteratureItem {
  return { id:r.id, fingerprint:r.fingerprint, sha256:r.sha256, path:r.path,
    filename:r.filename, fileSize:r.file_size, title:r.title, authors:r.authors,
    year:r.year, journal:r.journal, doi:r.doi, abstract:r.abstract, starred:!!r.starred,
    addedAt:r.added_at, updatedAt:r.updated_at,
    indexStatus:r.index_status === 'complete' ? 'complete' : r.index_status === 'partial' ? 'partial':'none',
    tags:[], collectionIds:[] };
}
function browserSnapshot():LibrarySnapshot {
  try { return JSON.parse(localStorage.getItem(BROWSER_KEY) || 'null') || { items:[], collections:[] }; }
  catch { return { items:[], collections:[] }; }
}
function browserSave(snapshot:LibrarySnapshot) { localStorage.setItem(BROWSER_KEY, JSON.stringify(snapshot)); }

let migration:Promise<unknown>|null=null;
async function nativeLibrary<T>(op:string,v?:unknown):Promise<T>{
 const bridge=electronBridge();if(!bridge)throw Error('Native library unavailable');
 if(!migration)migration=bridge.library('importPreview',browserSnapshot()).catch(e=>{migration=null;throw e;});
 await migration;return bridge.library(op,v) as Promise<T>;
}
export async function loadLibrary():Promise<LibrarySnapshot> {
  if(electronBridge()) return nativeLibrary<LibrarySnapshot>('load');
  if (!isTauri()) return browserSnapshot();
  await initLibrary(); const db = await getDatabase();
  const [rows, cols, tags, joins] = await Promise.all([
    db.select<ItemRow[]>('SELECT * FROM library_items ORDER BY updated_at DESC'),
    db.select<{id:string; name:string; parent_id:string|null}[]>('SELECT * FROM library_collections ORDER BY name COLLATE NOCASE'),
    db.select<{item_id:string; tag:string}[]>('SELECT item_id, tag FROM library_tags ORDER BY tag'),
    db.select<{item_id:string; collection_id:string}[]>('SELECT item_id, collection_id FROM library_memberships'),
  ]);
  const items = rows.map(fromRow); const byId = new Map(items.map(item=>[item.id,item]));
  tags.forEach(t=>byId.get(t.item_id)?.tags.push(t.tag));
  joins.forEach(t=>byId.get(t.item_id)?.collectionIds.push(t.collection_id));
  return {items,collections:cols.map(c=>({id:c.id,name:c.name,parentId:c.parent_id}))};
}

/** Atomically insert a new file reference; UNIQUE sha256 and path prevent duplicate file imports. */
export async function importLibraryItem(item:LiteratureItem):Promise<ImportStatus> {
  if(electronBridge()) return nativeLibrary<ImportStatus>('insert',item);
  if (!isTauri()) {
    const state=browserSnapshot();
    if (item.sha256 && state.items.some(x=>x.sha256===item.sha256)) return 'duplicate-hash';
    if (item.path && state.items.some(x=>x.path===item.path)) return 'duplicate-path';
    state.items.unshift(item); browserSave(state); return 'inserted';
  }
  await initLibrary(); const db=await getDatabase();
  if (item.sha256) {const rows=await db.select<{id:string}[]>('SELECT id FROM library_items WHERE sha256=$1 LIMIT 1',[item.sha256]);if(rows.length)return 'duplicate-hash';}
  if (item.path) {const rows=await db.select<{id:string}[]>('SELECT id FROM library_items WHERE path=$1 LIMIT 1',[item.path]);if(rows.length)return 'duplicate-path';}
  try {
    await db.execute(`INSERT INTO library_items (id,fingerprint,sha256,path,filename,file_size,title,authors,year,journal,doi,abstract,starred,index_status,added_at,updated_at)
      VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)`,[
      item.id,item.fingerprint,item.sha256,item.path,item.filename,item.fileSize,item.title,item.authors,item.year,item.journal,item.doi,item.abstract,
      item.starred?1:0,item.indexStatus,item.addedAt,item.updatedAt]);
    return 'inserted';
  } catch(error) {
    // Concurrent duplicate imports can race after SELECT. Only classify known UNIQUE violations.
    if (String(error).includes('UNIQUE constraint failed')) return 'duplicate-hash';
    throw error;
  }
}

export async function saveLibraryItem(item:LiteratureItem):Promise<void> {
  const changed={...item,updatedAt:NOW()};
  if(electronBridge()){await nativeLibrary('update',changed);return;}
  if (!isTauri()) {const state=browserSnapshot();state.items=state.items.map(i=>i.id===item.id?changed:i);browserSave(state);return;}
  await initLibrary();const db=await getDatabase();
  await db.execute(`UPDATE library_items SET fingerprint=$2,path=$3,filename=$4,title=$5,authors=$6,year=$7,journal=$8,doi=$9,abstract=$10,
    starred=$11,index_status=$12,updated_at=$13 WHERE id=$1`,[
    changed.id,changed.fingerprint,changed.path,changed.filename,changed.title,changed.authors,changed.year,changed.journal,changed.doi,
    changed.abstract,changed.starred?1:0,changed.indexStatus,changed.updatedAt]);
  await db.execute('DELETE FROM library_tags WHERE item_id=$1',[changed.id]);
  for(const tag of [...new Set(changed.tags.map(x=>x.trim()).filter(Boolean))]) {
    await db.execute('INSERT OR IGNORE INTO library_tags(item_id,tag) VALUES($1,$2)',[changed.id,tag]);
  }
  await db.execute('DELETE FROM library_memberships WHERE item_id=$1',[changed.id]);
  for(const collectionId of [...new Set(changed.collectionIds)]) {
    await db.execute('INSERT OR IGNORE INTO library_memberships(item_id,collection_id) VALUES($1,$2)',[changed.id,collectionId]);
  }
}
export async function deleteLibraryItem(id:string):Promise<void> {
  if(electronBridge()){await nativeLibrary('delete',id);return;}
  if (!isTauri()) {const state=browserSnapshot();state.items=state.items.filter(x=>x.id!==id);browserSave(state);return;}
  await initLibrary();const db=await getDatabase();
  await db.execute('DELETE FROM library_pages WHERE item_id=$1',[id]);
  try { await db.execute('DELETE FROM library_fts WHERE item_id=$1',[id]); } catch {/* optional */}
  await db.execute('DELETE FROM library_tags WHERE item_id=$1',[id]);
  await db.execute('DELETE FROM library_memberships WHERE item_id=$1',[id]);
  await db.execute('DELETE FROM library_items WHERE id=$1',[id]);
}
export async function createCollection(name:string,parentId:string|null=null):Promise<Collection> {
  const c={id:crypto.randomUUID(),name:name.trim(),parentId};
  if (!c.name) throw new Error('分类名称不能为空');
  if(electronBridge())return nativeLibrary<Collection>('createCollection',c);
  if (!isTauri()){const s=browserSnapshot();s.collections.push(c);browserSave(s);return c;}
  await initLibrary();const db=await getDatabase();
  await db.execute('INSERT INTO library_collections(id,name,parent_id) VALUES($1,$2,$3)',[c.id,c.name,c.parentId]);return c;
}
export async function removeCollection(id:string):Promise<void> {
  if(electronBridge()){await nativeLibrary('removeCollection',id);return;}
  if(!isTauri()){const s=browserSnapshot();s.collections=s.collections.filter(c=>c.id!==id);s.items.forEach(i=>i.collectionIds=i.collectionIds.filter(c=>c!==id));browserSave(s);return;}
  await initLibrary();const db=await getDatabase();
  await db.execute('UPDATE library_collections SET parent_id=NULL WHERE parent_id=$1',[id]);
  await db.execute('DELETE FROM library_memberships WHERE collection_id=$1',[id]);
  await db.execute('DELETE FROM library_collections WHERE id=$1',[id]);
}
export async function saveIndexedPage(itemId:string,page:number,text:string):Promise<void> {
  if(electronBridge()){await nativeLibrary('savePage',{itemId,page,text});return;}
  if(!isTauri()) return;
  await initLibrary();const db=await getDatabase();
  await db.execute('INSERT OR REPLACE INTO library_pages(item_id,page,content) VALUES($1,$2,$3)',[itemId,page,text]);
  try {
    await db.execute('DELETE FROM library_fts WHERE item_id=$1 AND page=$2',[itemId,page]);
    await db.execute('INSERT INTO library_fts(item_id,page,content) VALUES($1,$2,$3)',[itemId,page,text]);
  } catch {/* FTS5 optional */}
}
export async function clearIndexedPages(itemId:string):Promise<void> {
  if(electronBridge()){await nativeLibrary('clearPages',itemId);return;}
  if(!isTauri()) return;
  await initLibrary();const db=await getDatabase();
  await db.execute('DELETE FROM library_pages WHERE item_id=$1',[itemId]);
  try{ await db.execute('DELETE FROM library_fts WHERE item_id=$1',[itemId]); }catch{/* optional */}
}
/** Parameterized substring query works for Chinese and English, with or without FTS5. */
export async function searchIndexedItems(query:string):Promise<Set<string>> {
  if(electronBridge())return new Set(query.trim()?await nativeLibrary<string[]>('search',query.trim()):[]);
  if(!isTauri()||!query.trim()) return new Set();
  await initLibrary();const db=await getDatabase();
  const escaped=query.trim().replace(/\\/g,'\\\\').replace(/%/g,'\\%').replace(/_/g,'\\_');
  const rows=await db.select<{item_id:string}[]>("SELECT DISTINCT item_id FROM library_pages WHERE content LIKE $1 ESCAPE '\\' LIMIT 2000",[`%${escaped}%`]);
  return new Set(rows.map(r=>r.item_id));
}
