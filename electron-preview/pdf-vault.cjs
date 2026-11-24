'use strict';
const fs=require('node:fs/promises');
const path=require('node:path');
const {createHash,randomUUID}=require('node:crypto');
const MAX=100*1024*1024;
function createVault(root){
 const digest=b=>createHash('sha256').update(b).digest('hex');
 const validate=b=>{if(!(b instanceof Uint8Array)||b.byteLength<5||b.byteLength>MAX||Buffer.from(b.subarray(0,5)).toString()!=='%PDF-')throw Error('Invalid PDF or size limit');};
 const target=token=>{if(typeof token!=='string'||!/^vault:[a-f0-9]{64}$/.test(token))throw Error('Invalid PDF identifier');return path.join(root,token.slice(6)+'.pdf');};
 async function read(token){
  const file=target(token); const stat=await fs.lstat(file);
  if(!stat.isFile()||stat.size>MAX)throw Error('Invalid stored PDF');
  const handle=await fs.open(file,'r');
  try{const b=Buffer.alloc(stat.size);const {bytesRead}=await handle.read(b,0,b.length,0);if(bytesRead!==b.length)throw Error('Truncated PDF');validate(b);if(digest(b)!==token.slice(6))throw Error('PDF integrity mismatch');return new Uint8Array(b);}
  finally{await handle.close();}
 }
 async function store(bytes){
  validate(bytes);const token='vault:'+digest(bytes);await fs.mkdir(root,{recursive:true});
  const file=target(token);
  try{await read(token);return token;}catch(e){if(e.code!=='ENOENT')throw e;}
  const tmp=path.join(root,randomUUID()+'.tmp');
  try{await fs.writeFile(tmp,bytes,{flag:'wx'});await fs.rename(tmp,file);}finally{await fs.rm(tmp,{force:true});}
  return token;
 }
 return {store,read};
}
module.exports={createVault};
