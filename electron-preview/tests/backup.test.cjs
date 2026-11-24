const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs/promises'),os=require('node:os'),path=require('node:path');
const {createBackup,verifyBackup,DB_NAMES}=require('../backup.cjs');
const {createReaderDb}=require('../reader-db.cjs'),{createLibraryDb}=require('../library-db.cjs'),{createResearchDb}=require('../research-db.cjs'),{createVault}=require('../pdf-vault.cjs');
test('live WAL snapshots and vault integrity; corrupt or traversal manifest rejected',async()=>{
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'studio-backup-test-'));
 const profile=path.join(root,'profile'),parent=path.join(root,'backups');await fs.mkdir(profile);await fs.mkdir(parent);
 const stores=[createReaderDb,createLibraryDb,createResearchDb].map((fn,i)=>fn(path.join(profile,DB_NAMES[i])));
 try{
 const token=await createVault(path.join(profile,'pdf-vault')).store(Buffer.from('%PDF-1.4\nfixture'));
 stores[0].call('saveDocument',{fingerprint:'f',name:'paper.pdf',path:token,page:2,zoom:'fit',rotation:0,lastOpened:'2026-11-24'});
 await fs.mkdir(path.join(profile,'secrets'));await fs.writeFile(path.join(profile,'secrets','azure.dpapi'),'not backed up');
 const b=await createBackup({profile,parent,stores});assert.equal(b.pdfs,1);assert.equal(b.files,4);assert.equal(b.externalReferences,0);
 assert.equal((await verifyBackup(b.path)).files,4);
 const copy=createReaderDb(path.join(b.path,DB_NAMES[0]));try{assert.equal(copy.call('getDocument','f').page,2);}finally{copy.close();}
 const manifestPath=path.join(b.path,'manifest.json');const m=JSON.parse(await fs.readFile(manifestPath,'utf8'));
 await fs.writeFile(manifestPath,JSON.stringify({...m,files:[...m.files,{name:'../secret',size:0,sha256:'0'.repeat(64)}]}));await assert.rejects(()=>verifyBackup(b.path));
 await fs.writeFile(manifestPath,JSON.stringify(m));await fs.writeFile(path.join(b.path,'pdf-vault',token.slice(6)+'.pdf'),'damaged');await assert.rejects(()=>verifyBackup(b.path),/checksum/);
 await assert.rejects(()=>createBackup({profile,parent:profile,stores}),/outside/);
 }finally{stores.forEach(x=>x.close());await fs.rm(root,{recursive:true,force:true});}
});
