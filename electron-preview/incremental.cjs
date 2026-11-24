'use strict';
const fs=require('node:fs/promises'),sync=require('node:fs'),path=require('node:path'),{randomUUID}=require('node:crypto');
const {DB_NAMES,validName,fileHash,verifyBackup}=require('./backup.cjs');
const MAX=100*1024*1024*1024,PDF=/^[a-f0-9]{64}\.pdf$/;
const id=()=>new Date().toISOString().replace(/[:.]/g,'-')+'-'+randomUUID().slice(0,8);
async function dir(p){await fs.mkdir(p,{recursive:true});if(!(await fs.lstat(p)).isDirectory())throw Error('Invalid repository directory');return p;}
function inside(parent,child){const r=path.relative(parent,child);return r===''||(!path.isAbsolute(r)&&r!=='..'&&!r.startsWith('..'+path.sep));}
async function publish(source,objects,info){
 const target=path.join(objects,info.sha256);
 try{const old=await fileHash(target);if(old.sha256!==info.sha256||old.size!==info.size)throw Error('Corrupt existing backup object');return false;}catch(e){if(e.code!=='ENOENT')throw e;}
 const tmp=path.join(objects,'.partial-'+randomUUID());
 try{
  await fs.copyFile(source,tmp,sync.constants.COPYFILE_EXCL);
  const check=await fileHash(tmp);if(check.sha256!==info.sha256||check.size!==info.size)throw Error('Source changed during copy');
  // All writers publish the same verified content for a hash.
  await fs.rename(tmp,target);return true;
 }finally{await fs.rm(tmp,{force:true});}
}
async function createIncremental({profile,parent,stores}){
 const source=await fs.realpath(profile),dest=await fs.realpath(parent);
 if(inside(source,dest))throw Error('Backup must be outside active data');
 const repo=await dir(path.join(dest,'research-studio-incremental'));
 const objects=await dir(path.join(repo,'objects')),snaps=await dir(path.join(repo,'snapshots'));
 const name=id(),stage=path.join(snaps,'.partial-'+name),final=path.join(snaps,name);
 await fs.mkdir(stage);
 try{
  // Synchronous snapshots taken together before yielding to other main-process writes.
  for(let i=0;i<DB_NAMES.length;i++)stores[i].snapshot(path.join(stage,DB_NAMES[i]));
  const vault=path.join(source,'pdf-vault');
  let pdfs=[];try{if(!(await fs.lstat(vault)).isDirectory())throw Error('Invalid vault');pdfs=(await fs.readdir(vault)).filter(x=>PDF.test(x)).sort();}catch(e){if(e.code!=='ENOENT')throw e;}
  if(pdfs.length+3>50000)throw Error('Too many files');
  const files=[];let total=0,newObjects=0,reusedObjects=0,newBytes=0;
  for(const name of [...DB_NAMES,...pdfs.map(x=>'pdf-vault/'+x)]){
   const isPdf=name.startsWith('pdf-vault/'),file=path.join(isPdf?source:stage,name);
   const info=await fileHash(file,isPdf?100*1024*1024:MAX);
   if(isPdf&&info.sha256!==path.basename(name,'.pdf'))throw Error('PDF integrity mismatch');
   total+=info.size;if(total>MAX)throw Error('Snapshot exceeds 100 GiB');
   if(await publish(file,objects,info)){newObjects++;newBytes+=info.size;}else reusedObjects++;
   files.push({name,...info});
  }
  for(const name of DB_NAMES)await fs.rm(path.join(stage,name));
  await fs.writeFile(path.join(stage,'manifest.json'),JSON.stringify({format:'research-studio-backup',version:1,createdAt:new Date().toISOString(),files},null,2),{flag:'wx'});
  const checked=await verifyBackup(stage,objects);
  await fs.rename(stage,final);
  return {...checked,path:final,newObjects,reusedObjects,newBytes};
 }catch(e){await fs.rm(stage,{recursive:true,force:true});throw e;}
}
async function materializeIncremental(snapshot,parent){
 const src=await fs.realpath(snapshot),snapshots=path.dirname(src),repo=path.dirname(snapshots);
 if(path.basename(snapshots)!=='snapshots'||path.basename(repo)!=='research-studio-incremental')throw Error('Select an incremental snapshot folder');
 const objects=path.join(repo,'objects');await verifyBackup(src,objects);
 const destination=await fs.realpath(parent);
 if(inside(repo,destination))throw Error('Export outside the incremental repository');
 const manifest=JSON.parse(await fs.readFile(path.join(src,'manifest.json'),'utf8'));
 if(!Array.isArray(manifest.files)||manifest.files.length>50000||manifest.files.some(x=>!x||!validName(x.name)||!/^[a-f0-9]{64}$/.test(x.sha256)||!Number.isSafeInteger(x.size)||x.size<0)||manifest.files.reduce((n,x)=>n+x.size,0)>MAX)throw Error('Invalid snapshot manifest');
 const name=id(),stage=path.join(destination,'.partial-'+name),final=path.join(destination,'research-studio-backup-'+name);
 await fs.mkdir(stage);try{
  await fs.mkdir(path.join(stage,'pdf-vault'));
  for(const entry of manifest.files){
   const source=path.join(objects,entry.sha256),st=await fs.lstat(source);if(!st.isFile()||st.size!==entry.size)throw Error('Invalid object');
   await fs.copyFile(source,path.join(stage,entry.name),sync.constants.COPYFILE_EXCL);
  }
  await fs.writeFile(path.join(stage,'manifest.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
  const result=await verifyBackup(stage);await fs.rename(stage,final);return {...result,path:final};
 }catch(e){await fs.rm(stage,{recursive:true,force:true});throw e;}
}
module.exports={createIncremental,materializeIncremental};
