'use strict';
const fs=require('node:fs/promises'),{DatabaseSync}=require('node:sqlite');
// Fixed identifiers from V0.5.9 Tauri's existing data-store definitions.
const expected={
 documents:['fingerprint','name','path','page','zoom','rotation','last_opened'],
 annotations:['id','fingerprint','page','kind','color','quote','note','rects','created_at','updated_at'],
 library_items:['id','fingerprint','sha256','path','filename','file_size','title','authors','year','journal','doi','abstract','starred','index_status','added_at','updated_at'],
 library_collections:['id','name','parent_id'],library_memberships:['item_id','collection_id'],library_tags:['item_id','tag'],library_pages:['item_id','page','content'],
 research_clips:['id','title','body','source_url','item_id','fingerprint','page','tags','created_at','updated_at'],
 word_manuscripts:['id','title','content_json','source_path','page_size','margin_mm','created_at','updated_at'],
 translation_preferences:['id','region','source_lang','target_lang','monthly_limit','glossary'],
 translation_cache:['cache_key','source_text','translated_text','source_lang','target_lang','created_at'],
 translation_usage:['month','used_chars']
};
async function inspectLegacy(file){
 const st=await fs.lstat(file);if(!st.isFile()||st.size>10*1024*1024*1024||st.size<100)throw Error('Invalid legacy SQLite file');
 // Only read a closed DB or a legacy VACUUM INTO export; refuse live WAL files.
 for(const suffix of ['-wal','-journal'])try{await fs.lstat(file+suffix);throw Error('Close old software or choose its SQLite backup first');}catch(e){if(e.code!=='ENOENT')throw e;}
 const db=new DatabaseSync(file,{readOnly:true});
 try{
  db.exec('PRAGMA trusted_schema=OFF;PRAGMA query_only=ON');
  const result=db.prepare('PRAGMA quick_check').all();if(result.length!==1||Object.values(result[0])[0]!=='ok')throw Error('Legacy SQLite integrity check failed');
  const tables=new Set(db.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND sql NOT LIKE 'CREATE VIRTUAL TABLE%'").all().map(x=>x.name));
  if(!tables.has('documents')&&!tables.has('annotations'))throw Error('Not a recognized V0.5.9 database');
  const present=[],missing=[],incompatible=[];let records=0,externalReferences=0;
  for(const [name,columns] of Object.entries(expected)){
   if(!tables.has(name)){missing.push(name);continue;}
   const actual=new Set(db.prepare('PRAGMA table_info('+name+')').all().map(x=>x.name));
   const absent=columns.filter(x=>!actual.has(x));if(absent.length){incompatible.push({name,missing:absent});continue;}
   const count=db.prepare('SELECT count(*) AS n FROM '+name).get().n;records+=count;present.push({name,count});
   if(name==='documents'||name==='library_items')externalReferences+=db.prepare('SELECT count(*) AS n FROM '+name+" WHERE path IS NOT NULL AND path<>''").get().n;
  }
  return {present,missing,incompatible,records,externalReferences,compatible:incompatible.length===0};
 }finally{db.close();}
}
module.exports={inspectLegacy,expected};
