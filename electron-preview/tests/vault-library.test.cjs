const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs/promises'),os=require('node:os'),path=require('node:path');
const {createVault}=require('../pdf-vault.cjs'),{createLibraryDb}=require('../library-db.cjs');
test('vault integrity and SQLite library persistence',async()=>{
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'studio-vault-'));let db;
 try{
 const vault=createVault(path.join(root,'pdf'));const bytes=Buffer.from('%PDF-1.4\nfixture');
 const token=await vault.store(bytes);assert.equal(await vault.store(bytes),token);assert.deepEqual(Buffer.from(await vault.read(token)),bytes);
 await assert.rejects(()=>vault.read('../secret'));await assert.rejects(()=>vault.store(Buffer.from('bad')));
 const file=path.join(root,'library.sqlite');db=createLibraryDb(file);
 const item={id:'a',path:token,sha256:token.slice(6),fingerprint:null,filename:'a.pdf',fileSize:bytes.length,title:'Title',authors:'',year:null,journal:'',doi:'',abstract:'',starred:false,addedAt:'2026',updatedAt:'2026',indexStatus:'none',tags:[],collectionIds:[]};
 assert.equal(db.call('importPreview',{items:[item],collections:[]}),true);assert.equal(db.call('insert',{...item,id:'b'}),'duplicate-hash');
 db.call('savePage',{itemId:'a',page:1,text:'中文 scientific 100%'});db.close();db=createLibraryDb(file);
 assert.equal(db.call('load').items[0].title,'Title');assert.deepEqual(db.call('search','中文'),['a']);assert.deepEqual(db.call('search','100%'),['a']);assert.deepEqual(db.call('search','_'),[]);
 db.call('delete','a');assert.deepEqual(db.call('search','中文'),[]);
 await fs.writeFile(path.join(root,'pdf',token.slice(6)+'.pdf'),'%PDF-1.4\nchanged');await assert.rejects(()=>vault.read(token));
 }finally{db?.close();await fs.rm(root,{recursive:true,force:true});}
});
