const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {createReaderDb}=require('../reader-db.cjs');
test('SQLite persistence, fixed operations and transactional migration',()=>{
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'reader-db-')); const file=path.join(dir,'test.sqlite'); let db=createReaderDb(file);
 const doc={fingerprint:'f',name:'paper',path:null,page:2,zoom:'fit',rotation:0,lastOpened:'2026-11-24'};
 const ann={id:'a',fingerprint:'f',page:2,kind:'highlight',color:'yellow',quote:'text',note:'',rects:[[1,2,3,4]],createdAt:'2026',updatedAt:'2026'};
 try{
 assert.throws(()=>db.call('importPreview',{documents:[doc],annotations:[{}]}));assert.equal(db.call('getDocument','f'),null);
 assert.equal(db.call('importPreview',{documents:[doc],annotations:[ann]}),true);
 db.call('saveDocument',{...doc,page:3});assert.equal(db.call('importPreview',{documents:[doc],annotations:[]}),false);
 db.close();db=createReaderDb(file);assert.equal(db.call('getDocument','f').page,3);assert.deepEqual(db.call('getAnnotations','f'),[ann]);
 assert.throws(()=>db.call('sql','DROP TABLE documents'));assert.throws(()=>db.call('saveAnnotation',{...ann,rects:[[1,2,3,Infinity]]}));
 db.call('deleteAnnotation','a');assert.deepEqual(db.call('getAnnotations','f'),[]);
 }finally{db.close();fs.rmSync(dir,{recursive:true,force:true});}
});
