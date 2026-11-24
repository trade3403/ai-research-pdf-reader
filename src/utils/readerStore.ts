import { electronBridge } from './electronBridge';
import { isTauri } from '@tauri-apps/api/core';
import type { Annotation, AnnotationColor, AnnotationKind, PdfRect } from './annotations';
import { validRects } from './annotations';

/** Browser preview uses localStorage only; packaged Windows app uses SQLite in app-data. */
export type SavedDocument = {
  fingerprint: string;
  name: string;
  path: string | null;
  page: number;
  zoom: number | 'fit';
  rotation: number;
  lastOpened: string;
};

import type Database from '@tauri-apps/plugin-sql';
type Db = Database;
let connection: Promise<Db> | null = null;

export async function getDatabase(): Promise<Db> {
  if (!connection) {
    connection = (async () => {
      const { default: Database } = await import('@tauri-apps/plugin-sql');
      const db = await Database.load('sqlite:research-reader-v02.db');
      await db.execute(`CREATE TABLE IF NOT EXISTS annotations (
        id TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, page INTEGER NOT NULL,
        kind TEXT NOT NULL, color TEXT NOT NULL, quote TEXT NOT NULL,
        note TEXT NOT NULL, rects TEXT NOT NULL,
        created_at TEXT NOT NULL, updated_at TEXT NOT NULL
      )`);
      await db.execute('CREATE INDEX IF NOT EXISTS idx_annotations_document ON annotations(fingerprint, page)');
      await db.execute(`CREATE TABLE IF NOT EXISTS documents (
        fingerprint TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT,
        page INTEGER NOT NULL, zoom TEXT NOT NULL, rotation INTEGER NOT NULL,
        last_opened TEXT NOT NULL
      )`);
      return db;
    })().catch((error) => { connection = null; throw error; });
  }
  return connection;
}

const ANNOTATIONS_KEY = 'research-reader-v02-annotations';
const DOCUMENTS_KEY = 'research-reader-v02-documents';
function getBrowser<T>(key: string, fallback: T): T {
  try { return JSON.parse(localStorage.getItem(key) || 'null') ?? fallback; }
  catch { return fallback; }
}
function setBrowser<T>(key: string, value: T): void { localStorage.setItem(key, JSON.stringify(value)); }

let migrated: Promise<unknown>|null=null;
async function nativeReader<T>(op:string,value?:unknown):Promise<T> {
  const bridge=electronBridge();
  if(!bridge) throw new Error('Electron reader unavailable');
  if(!migrated) migrated=bridge.reader('importPreview',{
    documents:getBrowser<SavedDocument[]>(DOCUMENTS_KEY,[]),
    annotations:getBrowser<Annotation[]>(ANNOTATIONS_KEY,[])
  }).catch(error=>{migrated=null;throw error;});
  await migrated;
  return bridge.reader(op,value) as Promise<T>;
}

function safeAnnotation(ann: Annotation): Annotation {
  return { ...ann, rects: validRects(Array.isArray(ann.rects) ? ann.rects : []) };
}

type AnnotationRow = {
  id: string; fingerprint: string; page: number; kind: string; color: string;
  quote: string; note: string; rects: string; created_at: string; updated_at: string;
};
function fromAnnotationRow(row: AnnotationRow): Annotation {
  let rects: PdfRect[] = [];
  try { rects = JSON.parse(row.rects) as PdfRect[]; } catch { /* old/corrupt record */ }
  return safeAnnotation({
    id: row.id, fingerprint: row.fingerprint, page: row.page,
    kind: row.kind as AnnotationKind, color: row.color as AnnotationColor,
    quote: row.quote, note: row.note, rects,
    createdAt: row.created_at, updatedAt: row.updated_at,
  });
}

export async function getAnnotations(fingerprint: string): Promise<Annotation[]> {
  if (electronBridge()) return (await nativeReader<Annotation[]>('getAnnotations',fingerprint)).map(safeAnnotation);
  if (!isTauri()) return getBrowser<Annotation[]>(ANNOTATIONS_KEY, []).filter((a) => a.fingerprint === fingerprint).map(safeAnnotation);
  const db = await getDatabase();
  const rows = await db.select<AnnotationRow[]>('SELECT * FROM annotations WHERE fingerprint = $1 ORDER BY page, created_at', [fingerprint]);
  return rows.map(fromAnnotationRow);
}

export async function saveAnnotation(ann: Annotation): Promise<void> {
  if(electronBridge()){await nativeReader('saveAnnotation',ann);return;}
  if (!isTauri()) {
    const list = getBrowser<Annotation[]>(ANNOTATIONS_KEY, []).filter((a) => a.id !== ann.id);
    setBrowser(ANNOTATIONS_KEY, [...list, ann]);
    return;
  }
  const db = await getDatabase();
  await db.execute(`INSERT INTO annotations (id, fingerprint, page, kind, color, quote, note, rects, created_at, updated_at)
    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
    ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, color=excluded.color,
    note=excluded.note, rects=excluded.rects, updated_at=excluded.updated_at`, [
    ann.id, ann.fingerprint, ann.page, ann.kind, ann.color, ann.quote,
    ann.note, JSON.stringify(ann.rects), ann.createdAt, ann.updatedAt,
  ]);
}

export async function deleteAnnotation(id: string): Promise<void> {
  if(electronBridge()){await nativeReader('deleteAnnotation',id);return;}
  if (!isTauri()) {
    setBrowser(ANNOTATIONS_KEY, getBrowser<Annotation[]>(ANNOTATIONS_KEY, []).filter((a) => a.id !== id));
    return;
  }
  const db = await getDatabase();
  await db.execute('DELETE FROM annotations WHERE id = $1', [id]);
}

type DocumentRow = { fingerprint: string; name: string; path: string | null; page: number; zoom: string; rotation: number; last_opened: string };
function fromDocumentRow(row: DocumentRow): SavedDocument {
  const z = row.zoom === 'fit' ? 'fit' : Number(row.zoom);
  return { fingerprint: row.fingerprint, name: row.name, path: row.path,
    page: row.page, zoom: z === 'fit' || (typeof z === 'number' && Number.isFinite(z) && z > 0) ? z : 'fit',
    rotation: row.rotation, lastOpened: row.last_opened };
}

export async function listDocuments(): Promise<SavedDocument[]> {
  if(electronBridge()) return nativeReader<SavedDocument[]>('listDocuments');
  if (!isTauri()) return getBrowser<SavedDocument[]>(DOCUMENTS_KEY, []).sort((a, b) => b.lastOpened.localeCompare(a.lastOpened));
  const db = await getDatabase();
  const rows = await db.select<DocumentRow[]>('SELECT * FROM documents ORDER BY last_opened DESC LIMIT 40');
  return rows.map(fromDocumentRow);
}

export async function getDocument(fingerprint: string): Promise<SavedDocument | null> {
  if(electronBridge()) return nativeReader<SavedDocument|null>('getDocument',fingerprint);
  if (!isTauri()) return getBrowser<SavedDocument[]>(DOCUMENTS_KEY, []).find((a) => a.fingerprint === fingerprint) || null;
  const db = await getDatabase();
  const rows = await db.select<DocumentRow[]>('SELECT * FROM documents WHERE fingerprint = $1', [fingerprint]);
  return rows.length ? fromDocumentRow(rows[0]) : null;
}

export async function saveDocument(doc: SavedDocument): Promise<void> {
  if(electronBridge()){await nativeReader('saveDocument',doc);return;}
  if (!isTauri()) {
    const list = getBrowser<SavedDocument[]>(DOCUMENTS_KEY, []).filter((a) => a.fingerprint !== doc.fingerprint);
    setBrowser(DOCUMENTS_KEY, [doc, ...list].slice(0, 40));
    return;
  }
  const db = await getDatabase();
  await db.execute(`INSERT INTO documents (fingerprint, name, path, page, zoom, rotation, last_opened)
    VALUES ($1,$2,$3,$4,$5,$6,$7)
    ON CONFLICT(fingerprint) DO UPDATE SET name=excluded.name,
    path=COALESCE(excluded.path, documents.path), page=excluded.page,
    zoom=excluded.zoom, rotation=excluded.rotation, last_opened=excluded.last_opened`, [
    doc.fingerprint, doc.name, doc.path, doc.page, String(doc.zoom), doc.rotation, doc.lastOpened,
  ]);
}
