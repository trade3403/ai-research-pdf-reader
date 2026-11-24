'use strict';
const fs=require('node:fs');
const fsp=require('node:fs/promises');
const path=require('node:path');
const {createHash,randomUUID}=require('node:crypto');
const {DatabaseSync}=require('node:sqlite');
const DB_NAMES=['reader-v062.sqlite','library-v062.sqlite','research-v063.sqlite'];
const TABLES=[['documents','annotations','migrations'],['items','collections','pages','migrations'],['records','usage','migrations']];
const PDF=/^pdf-vault\/[a-f0-9]{64}\.pdf$/;
const MAX_FILES=50000,MAX_TOTAL=100*1024*1024*1024,MAX_MANIFEST=16*1024*1024;
function validName(name){return DB_NAMES.includes(name)||typeof name==='string'&&PDF.test(name);}
async function fileHash(file,max=MAX_TOTAL){
 const st=await fsp.lstat(file);if(!st.isFile()||st.size>max)throw Error('Invalid backup file');
 const h=createHash('sha256');let bytes=0;
 for await(const chunk of fs.createReadStream(file)){bytes+=chunk.length;if(bytes>max)throw Error('Backup file exceeds limit');h.update(chunk);}
 if(bytes!==st.size)throw Error('Backup file changed');
 return {size:bytes,sha256:h.digest('hex')};
}
function sqliteCheck(file,names){
 const db=new DatabaseSync(file,{readOnly:true});
 try{
  db.exec('PRAGMA trusted_schema=OFF;PRAGMA query_only=ON');
  const check=db.prepare('PRAGMA quick_check').all();
  if(check.length!==1||Object.values(check[0])[0]!=='ok')throw Error('SQLite integrity check failed');
  const tables=new Set(db.prepare("SELECT name FROM sqlite_schema WHERE type='table'").all().map(x=>x.name));
  for(const name of names)if(!tables.has(name))throw Error('Missing expected SQLite table: '+name);
 }finally{db.close();}
}
function isInside(parent,child){const rel=path.relative(parent,child);return rel===''||(!rel.startsWith('..'+path.sep)&&rel!=='..'&&!path.isAbsolute(rel));}
async function createBackup({profile,parent,stores}){
 const root=await fsp.realpath(profile),dest=await fsp.realpath(parent);
 if(isInside(root,dest))throw Error('Choose a backup directory outside the app data folder');
 const id=new Date().toISOString().replace(/[:.]/g,'-')+'-'+randomUUID().slice(0,8);
 const stage=path.join(dest,'.studio-backup-'+id+'.partial'),final=path.join(dest,'research-studio-backup-'+id);
 await fsp.mkdir(stage,{recursive:false});
 try{
  // No awaits in this block: main-process writes cannot interleave with database snapshots.
  for(let i=0;i<DB_NAMES.length;i++)stores[i].snapshot(path.join(stage,DB_NAMES[i]));
  const vault=path.join(root,'pdf-vault');
  let pdfs=[];if(fs.existsSync(vault)){if(!fs.lstatSync(vault).isDirectory())throw Error('Invalid vault directory');pdfs=fs.readdirSync(vault).filter(n=>/^[a-f0-9]{64}\.pdf$/.test(n));}
  if(pdfs.length+3>MAX_FILES)throw Error('Too many backup files');
  await fsp.mkdir(path.join(stage,'pdf-vault'));
  const files=[];let total=0;
  for(const name of [...DB_NAMES,...pdfs.map(n=>'pdf-vault/'+n).sort()]){
   const target=path.join(stage,name);
   if(PDF.test(name)){
    const source=path.join(root,name);
    const st=await fsp.lstat(source);if(!st.isFile()||st.size>100*1024*1024)throw Error('Invalid vault PDF');
    await fsp.copyFile(source,target,fs.constants.COPYFILE_EXCL);
   }
   const info=await fileHash(target,PDF.test(name)?100*1024*1024:MAX_TOTAL);
   if(PDF.test(name)&&info.sha256!==path.basename(name,'.pdf'))throw Error('Vault PDF integrity mismatch');
   total+=info.size;if(total>MAX_TOTAL)throw Error('Backup exceeds 100 GiB');
   files.push({name,...info});
  }
  const manifest={format:'research-studio-backup',version:1,createdAt:new Date().toISOString(),files};
  await fsp.writeFile(path.join(stage,'manifest.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
  const verified=await verifyBackup(stage);
  await fsp.rename(stage,final);
  return {...verified,path:final};
 }catch(e){await fsp.rm(stage,{recursive:true,force:true});throw e;}
}
async function verifyBackup(dir,objectRoot=null){
 const root=await fsp.realpath(dir);
 const manifestPath=path.join(root,'manifest.json'),st=await fsp.lstat(manifestPath);
 if(!st.isFile()||st.size>MAX_MANIFEST)throw Error('Invalid backup manifest');
 const m=JSON.parse(await fsp.readFile(manifestPath,'utf8'));
 if(m.format!=='research-studio-backup'||m.version!==1||!Array.isArray(m.files)||m.files.length>MAX_FILES)throw Error('Unsupported backup format');
 const names=new Set(),locations=new Map();let bytes=0,pdfs=0;
 if(objectRoot&&!(await fsp.lstat(objectRoot)).isDirectory())throw Error('Invalid object directory');
 if(!objectRoot&&m.files.some(x=>PDF.test(x?.name))&&!((await fsp.lstat(path.join(root,'pdf-vault'))).isDirectory()))throw Error('Invalid vault directory');
 for(const entry of m.files){
  if(!entry||!validName(entry.name)||names.has(entry.name)||!Number.isSafeInteger(entry.size)||entry.size<0||typeof entry.sha256!=='string'||!/^[a-f0-9]{64}$/.test(entry.sha256))throw Error('Invalid manifest entry');
  names.add(entry.name);bytes+=entry.size;if(bytes>MAX_TOTAL)throw Error('Backup exceeds limit');
  const f=objectRoot?path.join(objectRoot,entry.sha256):path.join(root,entry.name),actual=await fileHash(f,PDF.test(entry.name)?100*1024*1024:MAX_TOTAL);
  locations.set(entry.name,f);
  if(actual.size!==entry.size||actual.sha256!==entry.sha256)throw Error('Backup checksum mismatch: '+entry.name);
  if(PDF.test(entry.name)){if(actual.sha256!==path.basename(entry.name,'.pdf'))throw Error('Invalid vault content address');pdfs++;}
 }
 for(let i=0;i<DB_NAMES.length;i++){const name=DB_NAMES[i];if(!names.has(name))throw Error('Missing database: '+name);sqliteCheck(locations.get(name),TABLES[i]);}
 // Verify references from both stores. Legacy external paths are reported separately.
 let externalReferences=0;
 for(const [name,table] of [[DB_NAMES[0],'documents'],[DB_NAMES[1],'items']]){
  const db=new DatabaseSync(locations.get(name),{readOnly:true});
  try{db.exec('PRAGMA trusted_schema=OFF;PRAGMA query_only=ON');for(const row of db.prepare('SELECT payload FROM '+table).iterate()){
   const p=JSON.parse(row.payload).path;if(p===null)continue;
   if(typeof p!=='string')throw Error('Invalid stored PDF reference');
   if(!p.startsWith('vault:')){externalReferences++;continue;}
   if(!/^vault:[a-f0-9]{64}$/.test(p)||!names.has('pdf-vault/'+p.slice(6)+'.pdf'))throw Error('Missing referenced PDF');
  }}finally{db.close();}
 }
 return {files:names.size,pdfs,bytes,externalReferences};
}
module.exports={createBackup,verifyBackup,DB_NAMES,validName,sqliteCheck,TABLES,fileHash};
