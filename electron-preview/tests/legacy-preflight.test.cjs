const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs/promises'),os=require('node:os'),path=require('node:path'),{DatabaseSync}=require('node:sqlite'),{inspectLegacy}=require('../legacy-preflight.cjs');
test('legacy read-only preflight reports missing columns and external references',async()=>{
 const dir=await fs.mkdtemp(path.join(os.tmpdir(),'legacy-preflight-'));const file=path.join(dir,'legacy.sqlite');
 try{const db=new DatabaseSync(file);db.exec("CREATE TABLE documents(fingerprint TEXT,name TEXT,path TEXT,page INTEGER,zoom TEXT,rotation INTEGER,last_opened TEXT);INSERT INTO documents VALUES('f','paper','C:/paper.pdf',1,'fit',0,'2026');CREATE TABLE annotations(id TEXT);");db.close();
 const before=await fs.readFile(file);const result=await inspectLegacy(file);assert.equal(result.records,1);assert.equal(result.externalReferences,1);assert.equal(result.compatible,false);assert.equal(result.incompatible[0].name,'annotations');assert.ok(result.missing.includes('word_manuscripts'));assert.deepEqual(await fs.readFile(file),before);
 await fs.writeFile(file+'-wal','live');await assert.rejects(()=>inspectLegacy(file),/Close old software/);
 }finally{await fs.rm(dir,{recursive:true,force:true});}
});
