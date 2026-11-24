'use strict';
const fs=require('node:fs/promises');
const path=require('node:path');
const {randomUUID}=require('node:crypto');
function validateKey(key){if(typeof key!=='string'||!/^[!-~]{8,256}$/.test(key))throw Error('Invalid Azure Key');return key;}
function createKeyStore(file,safeStorage,platform=process.platform){
 function available(){if(platform!=='win32'||!safeStorage.isEncryptionAvailable())throw Error('Windows DPAPI unavailable; no plaintext fallback');}
 async function read(){available();let bytes;try{bytes=await fs.readFile(file);}catch(e){if(e.code==='ENOENT')return null;throw e;}if(bytes.length>16384)throw Error('Invalid encrypted key file');return validateKey(safeStorage.decryptString(bytes));}
 return {read,has:async()=>Boolean(await read()),remove:async()=>{await fs.rm(file,{force:true});},save:async(key)=>{available();const encrypted=safeStorage.encryptString(validateKey(key));await fs.mkdir(path.dirname(file),{recursive:true});const tmp=file+'.'+randomUUID()+'.tmp';try{await fs.writeFile(tmp,encrypted,{flag:'wx',mode:0o600});await fs.rename(tmp,file);}finally{await fs.rm(tmp,{force:true});}}};
}
function prepareBatch(v){
 if(!v||!Array.isArray(v.texts)||v.texts.length<1||v.texts.length>100||!['auto','en','zh-Hans'].includes(v.from)||!['en','zh-Hans'].includes(v.to)||typeof v.region!=='string'||!/^(global|[a-z0-9-]{2,45})$/.test(v.region))throw Error('Invalid translation request');
 let chars=0;for(const t of v.texts){if(typeof t!=='string'||!t.length||t.length>20000)throw Error('Invalid translation segment');chars+=Array.from(t).length;}if(chars>10000)throw Error('Translation batch too large');
 const url=new URL('https://api.cognitive.microsofttranslator.com/translate');url.searchParams.set('api-version','3.0');url.searchParams.set('to',v.to);if(v.from!=='auto')url.searchParams.set('from',v.from);
 return {url,chars,body:JSON.stringify(v.texts.map(Text=>({Text})))};
}
async function translateBatch(v,{keyStore,reserve,fetchImpl=fetch}){
 const b=prepareBatch(v);const key=await keyStore.read();if(!key)throw Error('Azure Key not configured');
 // Count submitted characters conservatively; do not refund failed/ambiguous requests.
 await reserve(b.chars);
 const headers={'Content-Type':'application/json','Ocp-Apim-Subscription-Key':key};if(v.region!=='global')headers['Ocp-Apim-Subscription-Region']=v.region;
 let response;try{response=await fetchImpl(b.url,{method:'POST',headers,body:b.body,redirect:'error',signal:AbortSignal.timeout(30000)});}catch{throw Error('Azure connection failed or timed out');}
 if(!response.ok){await response.body?.cancel().catch(()=>{});throw Error('Azure HTTP '+response.status);}
 const reader=response.body?.getReader();if(!reader)throw Error('Empty Azure response');
 let size=0;const chunks=[];try{while(true){const {done,value}=await reader.read();if(done)break;size+=value.byteLength;if(size>2*1024*1024){await reader.cancel();throw Error('Azure response exceeds limit');}chunks.push(Buffer.from(value));}}finally{reader.releaseLock();}
 let parsed;try{parsed=JSON.parse(Buffer.concat(chunks).toString('utf8'));}catch{throw Error('Invalid Azure response');}
 if(!Array.isArray(parsed)||parsed.length!==v.texts.length||parsed.some(x=>typeof x?.translations?.[0]?.text!=='string'))throw Error('Invalid Azure translation count');
 return {translations:parsed.map(x=>x.translations[0].text)};
}
module.exports={createKeyStore,prepareBatch,translateBatch};
