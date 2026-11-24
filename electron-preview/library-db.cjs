'use strict';
const {DatabaseSync}=require('node:sqlite');
const str=(x,n=512)=>{if(typeof x!=='string'||x.length>n)throw Error('Invalid library text');return x;};
const nullable=(x,n)=>x===null?null:str(x,n);
const arr=(x,n)=>{if(!Array.isArray(x)||x.length>n)throw Error('Invalid library list');return x.map(v=>str(v));};
function createLibraryDb(file){
 const db=new DatabaseSync(file);
 db.exec(`PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;
 CREATE TABLE IF NOT EXISTS items(id TEXT PRIMARY KEY,sha TEXT UNIQUE,path TEXT UNIQUE,payload TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS collections(id TEXT PRIMARY KEY,payload TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS pages(item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,page INTEGER NOT NULL,content TEXT NOT NULL,PRIMARY KEY(item_id,page));
 CREATE TABLE IF NOT EXISTS migrations(name TEXT PRIMARY KEY);`);
 const decode=rows=>rows.map(x=>JSON.parse(x.payload));
 const item=v=>{
  if(!v||!Number.isSafeInteger(v.fileSize)||v.fileSize<0||!['none','partial','complete'].includes(v.indexStatus)||typeof v.starred!=='boolean'||(v.year!==null&&!Number.isInteger(v.year)))throw Error('Invalid library item');
  return {id:str(v.id),fingerprint:nullable(v.fingerprint),sha256:nullable(v.sha256),path:nullable(v.path,32768),filename:str(v.filename,4096),fileSize:v.fileSize,title:str(v.title,16000),authors:str(v.authors,32000),year:v.year,journal:str(v.journal,16000),doi:str(v.doi,4096),abstract:str(v.abstract,250000),starred:v.starred,addedAt:str(v.addedAt,64),updatedAt:str(v.updatedAt,64),indexStatus:v.indexStatus,tags:arr(v.tags,1000),collectionIds:arr(v.collectionIds,1000)};
 };
 const col=v=>({id:str(v.id),name:str(v.name),parentId:nullable(v.parentId)});
 const load=()=>({items:decode(db.prepare('SELECT payload FROM items').all()),collections:decode(db.prepare('SELECT payload FROM collections').all())});
 function insert(v){const x=item(v);if(x.sha256&&db.prepare('SELECT id FROM items WHERE sha=?').get(x.sha256))return 'duplicate-hash';if(x.path&&db.prepare('SELECT id FROM items WHERE path=?').get(x.path))return 'duplicate-path';db.prepare('INSERT INTO items VALUES(?,?,?,?)').run(x.id,x.sha256,x.path,JSON.stringify(x));return 'inserted';}
 function update(v){const x=item(v);db.prepare('UPDATE items SET sha=?,path=?,payload=? WHERE id=?').run(x.sha256,x.path,JSON.stringify(x),x.id);}
 function addCol(v){const x=col(v);db.prepare('INSERT INTO collections VALUES(?,?)').run(x.id,JSON.stringify(x));return x;}
 function tx(fn){db.exec('BEGIN IMMEDIATE');try{const out=fn();db.exec('COMMIT');return out;}catch(e){db.exec('ROLLBACK');throw e;}}
 function call(op,v){
  switch(op){
   case 'load':return load();
   case 'importPreview':if(db.prepare('SELECT name FROM migrations WHERE name=?').get('v062-library'))return false;
    if(!v||!Array.isArray(v.items)||!Array.isArray(v.collections)||v.items.length>10000||v.collections.length>1000||JSON.stringify(v).length>32*1024*1024)throw Error('Invalid library migration');
    return tx(()=>{for(const c of v.collections){const x=col(c);if(!db.prepare('SELECT id FROM collections WHERE id=?').get(x.id))addCol(x);}for(const i of v.items){const x=item(i);if(!db.prepare('SELECT id FROM items WHERE id=?').get(x.id))insert(x);}db.prepare('INSERT INTO migrations VALUES(?)').run('v062-library');return true;});
   case 'insert':return insert(v);
   case 'update':update(v);return null;
   case 'delete':db.prepare('DELETE FROM items WHERE id=?').run(str(v));return null;
   case 'createCollection':return addCol(v);
   case 'removeCollection':return tx(()=>{const id=str(v),s=load();for(const c of s.collections)if(c.parentId===id)db.prepare('UPDATE collections SET payload=? WHERE id=?').run(JSON.stringify({...c,parentId:null}),c.id);for(const i of s.items)if(i.collectionIds.includes(id))update({...i,collectionIds:i.collectionIds.filter(x=>x!==id)});db.prepare('DELETE FROM collections WHERE id=?').run(id);return null;});
   case 'savePage':if(!v||!Number.isInteger(v.page)||v.page<1||v.page>10000000)throw Error('Invalid page');db.prepare('INSERT OR REPLACE INTO pages VALUES(?,?,?)').run(str(v.itemId),v.page,str(v.text,2000000));return null;
   case 'clearPages':db.prepare('DELETE FROM pages WHERE item_id=?').run(str(v));return null;
   case 'search':return db.prepare('SELECT DISTINCT item_id FROM pages WHERE instr(lower(content),lower(?))>0 LIMIT 2000').all(str(v,4096)).map(x=>x.item_id);
   default:throw Error('Unsupported library operation');
  }
 }
 return {call,snapshot:target=>db.prepare('VACUUM INTO ?').run(target),close:()=>db.close()};
}
module.exports={createLibraryDb};
