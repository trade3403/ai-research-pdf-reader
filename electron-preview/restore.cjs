'use strict';
const fs=require('node:fs/promises');
const path=require('node:path');
const {randomUUID}=require('node:crypto');
const {DatabaseSync}=require('node:sqlite');
const {verifyBackup,DB_NAMES,validName,sqliteCheck,TABLES}=require('./backup.cjs');
const ID=/^generation-[a-f0-9-]{36}$/;
const validId=id=>id==='original'||typeof id==='string'&&ID.test(id);
const pointer='active-research-data.json',pending='pending-research-restore.json';
async function readJson(file){try{const st=await fs.lstat(file);if(!st.isFile()||st.size>65536)throw Error('Invalid restore state');return JSON.parse(await fs.readFile(file,'utf8'));}catch(e){if(e.code==='ENOENT')return null;throw e;}}
async function atomicJson(file,value){const tmp=file+'.'+randomUUID()+'.tmp';try{const handle=await fs.open(tmp,'wx');try{await handle.writeFile(JSON.stringify(value));await handle.sync();}finally{await handle.close();}await fs.rename(tmp,file);}finally{await fs.rm(tmp,{force:true});}}
async function generationRoot(profile,id){
 if(!validId(id))throw Error('Invalid data generation');
 if(id==='original')return profile;
 const dir=path.join(profile,'data-generations');
 if(!(await fs.lstat(dir)).isDirectory())throw Error('Invalid generations folder');
 const target=path.join(dir,id);
 if(!(await fs.lstat(target)).isDirectory())throw Error('Missing data generation');
 return target;
}
async function state(profile){const p=await readJson(path.join(profile,pointer));if(!p)return {version:1,active:'original',previous:null};if(p.version!==1||!validId(p.active)||(p.previous!==null&&!validId(p.previous))||p.active===p.previous)throw Error('Invalid active data pointer');return p;}
async function checkGeneration(root){for(let i=0;i<DB_NAMES.length;i++){const file=path.join(root,DB_NAMES[i]);if(!(await fs.lstat(file)).isFile())throw Error('Missing data file');sqliteCheck(file,TABLES[i]);}}
async function stageRestore(profile,source){
 const p=await state(profile);if(await readJson(path.join(profile,pending)))throw Error('A restore is already scheduled');
 const sourceRoot=await fs.realpath(source);const verified=await verifyBackup(sourceRoot);
 // Re-read and validate every name before copying, then verify the copied snapshot again.
 const manifestFile=path.join(sourceRoot,'manifest.json');const manifestStat=await fs.lstat(manifestFile);if(!manifestStat.isFile()||manifestStat.size>16*1024*1024)throw Error('Invalid manifest');
 const manifest=JSON.parse(await fs.readFile(manifestFile,'utf8'));if(!Array.isArray(manifest.files)||manifest.files.length>50000||manifest.files.some(x=>!x||!validName(x.name)||!Number.isSafeInteger(x.size)||x.size<0)||manifest.files.reduce((n,x)=>n+x.size,0)>100*1024*1024*1024)throw Error('Invalid manifest');
 const parent=path.join(profile,'data-generations');await fs.mkdir(parent,{recursive:true});
 if(!(await fs.lstat(parent)).isDirectory())throw Error('Invalid generations folder');
 const id='generation-'+randomUUID(),target=path.join(parent,id);
 await fs.mkdir(target);
 try{
  await fs.mkdir(path.join(target,'pdf-vault'));
  for(const entry of manifest.files){const f=path.join(sourceRoot,entry.name),st=await fs.lstat(f);if(!st.isFile()||st.size!==entry.size)throw Error('Backup changed during staging');await fs.copyFile(f,path.join(target,entry.name),require('node:fs').constants.COPYFILE_EXCL);}
  await fs.writeFile(path.join(target,'manifest.json'),JSON.stringify(manifest),{flag:'wx'});
  const result=await verifyBackup(target);
  await atomicJson(path.join(profile,pending),{version:1,kind:'restore',from:p.active,to:id});
  return {...result,scheduled:true};
 }catch(e){await fs.rm(target,{recursive:true,force:true});throw e;}
}
async function stageRollback(profile){
 const p=await state(profile);
 if(await readJson(path.join(profile,pending)))throw Error('A restore is already scheduled');
 if(!p.previous)throw Error('No previous data generation available');
 await checkGeneration(await generationRoot(profile,p.previous));
 await atomicJson(path.join(profile,pending),{version:1,kind:'rollback',from:p.active,to:p.previous});
 return {scheduled:true};
}
function preserveUsage(oldRoot,newRoot){
 const old=new DatabaseSync(path.join(oldRoot,DB_NAMES[2]),{readOnly:true});
 const next=new DatabaseSync(path.join(newRoot,DB_NAMES[2]));
 try{
  old.exec('PRAGMA trusted_schema=OFF;PRAGMA query_only=ON');
  next.exec('PRAGMA trusted_schema=OFF;BEGIN IMMEDIATE');
  try{const put=next.prepare('INSERT INTO usage(month,used) VALUES(?,?) ON CONFLICT(month) DO UPDATE SET used=max(usage.used,excluded.used)');for(const row of old.prepare('SELECT month,used FROM usage').iterate())put.run(row.month,row.used);next.exec('COMMIT');}
  catch(e){next.exec('ROLLBACK');throw e;}
 }finally{next.close();old.close();}
}
function sealImports(root){
 const marks=[['v061-reader'],['v062-library'],['clip','word','translation']];
 for(let i=0;i<DB_NAMES.length;i++){const db=new DatabaseSync(path.join(root,DB_NAMES[i]));try{db.exec('PRAGMA trusted_schema=OFF');for(const name of marks[i])db.prepare('INSERT OR IGNORE INTO migrations(name) VALUES(?)').run(name);}finally{db.close();}}
}
async function activatePending(profile){
 const p=await state(profile),current=await generationRoot(profile,p.active);
 const file=path.join(profile,pending);let job;
 try{
  job=await readJson(file);if(!job)return {root:current,previous:p.previous,changed:false};
  if(job.version!==1||!['restore','rollback'].includes(job.kind)||!validId(job.from)||!validId(job.to)||job.from===job.to)throw Error('Invalid pending restore');
  // If process died after pointer commit, cleanup only; never re-verify a live modified database.
  if(p.active===job.to&&p.previous===job.from){await fs.rm(file,{force:true});return {root:current,previous:p.previous,changed:false};}
  if(p.active!==job.from)throw Error('Restore source generation changed');
  if(job.kind==='restore'&&!ID.test(job.to))throw Error('Invalid restore target');
  if(job.kind==='rollback'&&job.to!==p.previous)throw Error('Invalid rollback target');
  const next=await generationRoot(profile,job.to);
  if(job.kind==='restore')await verifyBackup(next);else await checkGeneration(next);
  await checkGeneration(current);
  // Maintain the maximum locally counted Azure usage across generations.
  preserveUsage(current,next);sealImports(next);
  await checkGeneration(next);
  await atomicJson(path.join(profile,pointer),{version:1,active:job.to,previous:p.active});
  // Pointer commit is final: failure to remove pending is harmless, handled on next startup.
  await fs.rm(file,{force:true}).catch(()=>{});
  return {root:next,previous:p.active,changed:true};
 }catch(e){
  // Current generation stays selected if validation or pointer write failed.
  await fs.rename(file,path.join(profile,'failed-restore-'+randomUUID()+'.json')).catch(()=>{});
  return {root:current,previous:p.previous,changed:false,error:String(e.message||e)};
 }
}
module.exports={stageRestore,stageRollback,activatePending,state};
