'use strict';
const { DatabaseSync } = require('node:sqlite');
const string = (v,max=512) => { if(typeof v !== 'string' || v.length>max) throw new Error('Invalid text'); return v; };
const integer = (v,min,max) => { if(!Number.isInteger(v)||v<min||v>max) throw new Error('Invalid integer'); return v; };
function createReaderDb(file) {
  const db = new DatabaseSync(file);
  db.exec(`PRAGMA journal_mode=WAL; PRAGMA busy_timeout=3000;
    CREATE TABLE IF NOT EXISTS documents(fingerprint TEXT PRIMARY KEY, last_opened TEXT NOT NULL, payload TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS annotations(id TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, payload TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS annotations_doc ON annotations(fingerprint);
    CREATE TABLE IF NOT EXISTS migrations(name TEXT PRIMARY KEY);`);
  const decode = rows => rows.map(r=>JSON.parse(r.payload));
  const doc = v => {
    if(!v || typeof v!=='object') throw new Error('Invalid document');
    const zoom = v.zoom==='fit' ? 'fit' : v.zoom;
    if(zoom!=='fit' && (!Number.isFinite(zoom)||zoom<=0||zoom>100)) throw new Error('Invalid zoom');
    return {fingerprint:string(v.fingerprint),name:string(v.name,4096),path:v.path===null?null:string(v.path,32768),page:integer(v.page,1,10000000),zoom,rotation:integer(v.rotation,-36000,36000),lastOpened:string(v.lastOpened,64)};
  };
  const ann = v => {
    if(!v||typeof v!=='object'||!['highlight','underline','strike'].includes(v.kind)||!['yellow','green','blue','pink'].includes(v.color)||!Array.isArray(v.rects)||v.rects.length>10000||!v.rects.every(r=>Array.isArray(r)&&r.length===4&&r.every(n=>Number.isFinite(n)&&Math.abs(n)<=1e9))) throw new Error('Invalid annotation');
    return {id:string(v.id),fingerprint:string(v.fingerprint),page:integer(v.page,1,10000000),kind:v.kind,color:v.color,quote:string(v.quote,250000),note:string(v.note,250000),rects:v.rects,createdAt:string(v.createdAt,64),updatedAt:string(v.updatedAt,64)};
  };
  function saveDoc(v,ignore=false) {
    const x=doc(v);
    db.prepare(ignore?'INSERT OR IGNORE INTO documents VALUES(?,?,?)':'INSERT INTO documents VALUES(?,?,?) ON CONFLICT(fingerprint) DO UPDATE SET last_opened=excluded.last_opened,payload=excluded.payload').run(x.fingerprint,x.lastOpened,JSON.stringify(x));
  }
  function saveAnn(v,ignore=false) {
    const x=ann(v);
    db.prepare(ignore?'INSERT OR IGNORE INTO annotations VALUES(?,?,?)':'INSERT INTO annotations VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET fingerprint=excluded.fingerprint,payload=excluded.payload').run(x.id,x.fingerprint,JSON.stringify(x));
  }
  function call(op,value) {
    switch(op) {
      case 'listDocuments': return decode(db.prepare('SELECT payload FROM documents ORDER BY last_opened DESC LIMIT 40').all());
      case 'getDocument': return decode(db.prepare('SELECT payload FROM documents WHERE fingerprint=?').all(string(value)))[0]||null;
      case 'saveDocument': saveDoc(value); return null;
      case 'getAnnotations': return decode(db.prepare('SELECT payload FROM annotations WHERE fingerprint=? LIMIT 10001').all(string(value)));
      case 'saveAnnotation': saveAnn(value); return null;
      case 'deleteAnnotation': db.prepare('DELETE FROM annotations WHERE id=?').run(string(value)); return null;
      case 'importPreview': {
        if(db.prepare('SELECT name FROM migrations WHERE name=?').get('v061-reader')) return false;
        if(!value||!Array.isArray(value.documents)||!Array.isArray(value.annotations)||value.documents.length>1000||value.annotations.length>10000||JSON.stringify(value).length>16*1024*1024) throw new Error('Invalid migration batch');
        db.exec('BEGIN IMMEDIATE');
        try {for(const v of value.documents) saveDoc(v,true); for(const v of value.annotations) saveAnn(v,true); db.prepare('INSERT INTO migrations VALUES(?)').run('v061-reader');db.exec('COMMIT');return true;}
        catch(e){db.exec('ROLLBACK');throw e;}
      }
      default: throw new Error('Unsupported reader operation');
    }
  }
  return {call,close:()=>db.close()};
}
module.exports={createReaderDb};
