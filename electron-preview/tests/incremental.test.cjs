const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs/promises'),os=require('node:os'),path=require('node:path');
const {createIncremental,materializeIncremental}=require('../incremental.cjs'),{DB_NAMES,verifyBackup}=require('../backup.cjs'),{createReaderDb}=require('../reader-db.cjs'),{createLibraryDb}=require('../library-db.cjs'),{createResearchDb}=require('../research-db.cjs'),{createVault}=require('../pdf-vault.cjs');
test('unchanged objects reused; independent export verified; corrupt shared object refused',async()=>{
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'incremental-'));const profile=path.join(root,'profile'),parent=path.join(root,'backup');await fs.mkdir(profile);await fs.mkdir(parent);
 const stores=[createReaderDb,createLibraryDb,createResearchDb].map((f,i)=>f(path.join(profile,DB_NAMES[i])));
 try{
 const token=await createVault(path.join(profile,'pdf-vault')).store(Buffer.from('%PDF-1.4\nfixture'));
 const doc={fingerprint:'f',name:'paper.pdf',path:token,page:1,zoom:'fit',rotation:0,lastOpened:'2026'};
 stores[0].call('saveDocument',doc);const options={profile,parent,stores};
 const first=await createIncremental(options);assert.equal(first.newObjects,4);
 stores[0].call('saveDocument',{...doc,page:2});
 const second=await createIncremental(options);assert.ok(second.reusedObjects>=1);assert.ok(second.newObjects<4);
 const exported=await materializeIncremental(second.path,parent);assert.equal((await verifyBackup(exported.path)).pdfs,1);
 const reader=createReaderDb(path.join(exported.path,DB_NAMES[0]));try{assert.equal(reader.call('getDocument','f').page,2);}finally{reader.close();}
 await fs.writeFile(path.join(parent,'research-studio-incremental','objects',token.slice(6)),'corrupt');
 await assert.rejects(()=>materializeIncremental(first.path,parent));await assert.rejects(()=>createIncremental(options),/Corrupt/);
 }finally{stores.forEach(x=>x.close());await fs.rm(root,{recursive:true,force:true});}
});
