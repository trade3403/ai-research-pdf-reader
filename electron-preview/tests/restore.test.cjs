const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs/promises'),os=require('node:os'),path=require('node:path');
const {createBackup,DB_NAMES}=require('../backup.cjs'),{stageRestore,stageRollback,activatePending,state}=require('../restore.cjs');
const {createReaderDb}=require('../reader-db.cjs'),{createLibraryDb}=require('../library-db.cjs'),{createResearchDb}=require('../research-db.cjs');
test('restore switches generations only after verification, old state retained, rollback and corruption recovery',async()=>{
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'studio-restore-'));const profile=path.join(root,'profile'),parent=path.join(root,'backup');await fs.mkdir(profile);await fs.mkdir(parent);
 const stores=[createReaderDb,createLibraryDb,createResearchDb].map((fn,i)=>fn(path.join(profile,DB_NAMES[i])));
 const doc={fingerprint:'f',name:'paper.pdf',path:null,page:1,zoom:'fit',rotation:0,lastOpened:'2026'};
 try{
 stores[0].call('saveDocument',doc);stores[2].call('translation.reserve',{month:'2026-11',chars:50,limit:1000});
 const backup=await createBackup({profile,parent,stores});
 stores[0].call('saveDocument',{...doc,page:9});stores[2].call('translation.reserve',{month:'2026-11',chars:50,limit:1000});
 const staged=await stageRestore(profile,backup.path);assert.equal(staged.scheduled,true);assert.equal((await state(profile)).active,'original');
 await assert.rejects(()=>stageRestore(profile,backup.path),/already/);
 stores.forEach(x=>x.close());
 const active=await activatePending(profile);assert.equal(active.changed,true);assert.notEqual(active.root,profile);
 let db=createReaderDb(path.join(active.root,DB_NAMES[0]));assert.equal(db.call('getDocument','f').page,1);db.close();
 db=createResearchDb(path.join(active.root,DB_NAMES[2]));assert.equal(db.call('translation.usage','2026-11'),100);db.close();
 await stageRollback(profile);const rolled=await activatePending(profile);assert.equal(rolled.root,profile);
 db=createReaderDb(path.join(profile,DB_NAMES[0]));assert.equal(db.call('getDocument','f').page,9);db.close();
 await stageRestore(profile,backup.path);
 const pending=JSON.parse(await fs.readFile(path.join(profile,'pending-research-restore.json'),'utf8'));
 await fs.writeFile(path.join(profile,'data-generations',pending.to,DB_NAMES[0]),'corrupt');
 const rejected=await activatePending(profile);assert.equal(rejected.root,profile);assert.ok(rejected.error);assert.equal((await state(profile)).active,'original');
 }finally{for(const x of stores)try{x.close();}catch{}await fs.rm(root,{recursive:true,force:true});}
});
