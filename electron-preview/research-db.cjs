'use strict';
const {DatabaseSync}=require('node:sqlite');
const str=(v,max=512,min=0)=>{if(typeof v!=='string'||v.length>max||v.length<min)throw Error('Invalid text');return v;};
const nullable=(v,max)=>v===null?null:str(v,max);
const integer=(v,min,max)=>{if(!Number.isSafeInteger(v)||v<min||v>max)throw Error('Invalid integer');return v;};
function clip(v){
 const u=new URL(str(v.sourceUrl,4096));
 if(u.protocol!=='https:'||!['chatgpt.com','chat.openai.com'].includes(u.hostname)||u.username||u.password)throw Error('Invalid source');
 return {id:str(v.id,512,1),title:str(v.title.trim(),240,1),body:str(v.body.trim(),250000,1),sourceUrl:u.href,itemId:nullable(v.itemId,512),fingerprint:nullable(v.fingerprint,512),page:v.page===null?null:integer(v.page,1,1000000),tags:str(v.tags,400),createdAt:str(v.createdAt,64),updatedAt:str(v.updatedAt,64)};
}
function manuscript(v){
 if(!['A4','Letter'].includes(v.pageSize)||!['numeric','author-year'].includes(v.citationStyle)||![1,1.15,1.5,2].includes(v.lineSpacing)||!Number.isFinite(v.marginMm)||v.marginMm<10||v.marginMm>45||!v.content||v.content.type!=='doc')throw Error('Invalid manuscript');
 const json=JSON.stringify(v.content);str(json,8000000);
 return {id:str(v.id,512,1),title:str(v.title.trim(),240,1),content:JSON.parse(json),sourcePath:nullable(v.sourcePath,32768),pageSize:v.pageSize,marginMm:v.marginMm,citationStyle:v.citationStyle,lineSpacing:v.lineSpacing,headerText:str(v.headerText,150),footerText:str(v.footerText,150),createdAt:str(v.createdAt,64),updatedAt:str(v.updatedAt,64)};
}
function settings(v){
 if(!/^(global|[a-z0-9-]{2,45})$/.test(str(v.region,45))||!['auto','en','zh-Hans'].includes(v.from)||!['zh-Hans','en'].includes(v.to)||!Array.isArray(v.glossary)||v.glossary.length>100)throw Error('Invalid translation settings');
 return {region:v.region,from:v.from,to:v.to,monthlyLimit:integer(v.monthlyLimit,1000,2000000),glossary:v.glossary.map(x=>({source:str(x.source,80,1),target:str(x.target,100,1)}))};
}
const month=v=>{if(typeof v!=='string'||!/^\d{4}-(0[1-9]|1[0-2])$/.test(v))throw Error('Invalid month');return v;};
const hash=v=>{if(typeof v!=='string'||!/^[a-f0-9]{64}$/.test(v))throw Error('Invalid cache key');return v;};
function translation(v){return {cacheKey:hash(v.cacheKey),source:str(v.source,25000),translated:str(v.translated,1000000),from:str(v.from,16),to:str(v.to,16),createdAt:str(v.createdAt,64)};}
function createResearchDb(file){
 const db=new DatabaseSync(file);
 db.exec(`PRAGMA journal_mode=WAL;PRAGMA busy_timeout=3000;
 CREATE TABLE IF NOT EXISTS records(kind TEXT NOT NULL,id TEXT NOT NULL,updated TEXT NOT NULL,payload TEXT NOT NULL,PRIMARY KEY(kind,id));
 CREATE INDEX IF NOT EXISTS recent ON records(kind,updated DESC);
 CREATE TABLE IF NOT EXISTS usage(month TEXT PRIMARY KEY,used INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS migrations(name TEXT PRIMARY KEY);`);
 const get=(k,id)=>{const row=db.prepare('SELECT payload FROM records WHERE kind=? AND id=?').get(k,id);return row?JSON.parse(row.payload):null;};
 const list=(k,n)=>db.prepare('SELECT payload FROM records WHERE kind=? ORDER BY updated DESC LIMIT ?').all(k,n).map(x=>JSON.parse(x.payload));
 function write(k,id,updated,value,ignore=false){db.prepare(ignore?'INSERT OR IGNORE INTO records VALUES(?,?,?,?)':'INSERT INTO records VALUES(?,?,?,?) ON CONFLICT(kind,id) DO UPDATE SET updated=excluded.updated,payload=excluded.payload').run(k,id,updated,JSON.stringify(value));}
 function save(k,v,ignore=false){const x=k==='clip'?clip(v):manuscript(v);const old=get(k,x.id);if(old)x.createdAt=old.createdAt;write(k,x.id,x.updatedAt,x,ignore);return x;}
 function tx(fn){db.exec('BEGIN IMMEDIATE');try{const x=fn();db.exec('COMMIT');return x;}catch(e){db.exec('ROLLBACK');throw e;}}
 function migrate(name,v,fn){if(db.prepare('SELECT name FROM migrations WHERE name=?').get(name))return false;if(JSON.stringify(v).length>32*1024*1024)throw Error('Migration exceeds size limit');return tx(()=>{fn();db.prepare('INSERT INTO migrations VALUES(?)').run(name);return true;});}
 function call(op,v){
 switch(op){
 case 'clip.import':case 'word.import':{const k=op.startsWith('clip')?'clip':'word';return migrate(k,v,()=>{if(!Array.isArray(v)||v.length>10000)throw Error('Invalid migration');for(const x of v)save(k,x,true);});}
 case 'clip.list':return list('clip',10000);
 case 'word.list':return list('word',400);
 case 'clip.save':return save('clip',v);
 case 'word.save':return save('word',v);
 case 'clip.delete':case 'word.delete':db.prepare('DELETE FROM records WHERE kind=? AND id=?').run(op.startsWith('clip')?'clip':'word',str(v));return null;
 case 'translation.import':return migrate('translation',v,()=>{const s=settings(v.settings);write('settings','singleton','',s,true);if(!Array.isArray(v.cache)||v.cache.length>10000||!v.usage||typeof v.usage!=='object')throw Error('Invalid cache migration');for(const x of v.cache){const t=translation(x);write('cache',t.cacheKey,t.createdAt,t,true);}for(const [m,n] of Object.entries(v.usage))db.prepare('INSERT OR IGNORE INTO usage VALUES(?,?)').run(month(m),integer(n,0,1000000000));});
 case 'translation.settings':return get('settings','singleton');
 case 'translation.saveSettings':{const x=settings(v);write('settings','singleton','',x);return x;}
 case 'translation.usage':return db.prepare('SELECT used FROM usage WHERE month=?').get(month(v))?.used||0;
 case 'translation.reserve':{const m=month(v.month),chars=integer(v.chars,0,30000),limit=integer(v.limit,1000,2000000);return tx(()=>{db.prepare('INSERT OR IGNORE INTO usage VALUES(?,0)').run(m);const result=db.prepare('UPDATE usage SET used=used+? WHERE month=? AND used+?<=?').run(chars,m,chars,limit);if(result.changes!==1)throw Error('Monthly translation budget exceeded');return null;});}
 case 'translation.cache':return get('cache',hash(v));
 case 'translation.saveCache':{const t=translation(v);write('cache',t.cacheKey,t.createdAt,t);return null;}
 default:throw Error('Unsupported research operation');
 }}
 return {call,close:()=>db.close()};
}
module.exports={createResearchDb};
