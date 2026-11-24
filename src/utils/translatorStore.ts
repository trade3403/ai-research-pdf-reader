import { electronBridge } from './electronBridge';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getDatabase } from './readerStore';
import { assembleResearchTranslation, splitResearchMarkup, translationPayload } from './translationMarkup';
import type { GlossaryEntry } from './translationMarkup';

export type { GlossaryEntry };
export type TranslatorSettings = { region: string; from: 'auto'|'en'|'zh-Hans'; to: 'zh-Hans'|'en'; monthlyLimit: number; glossary: GlossaryEntry[] };
export type SavedTranslation = { cacheKey: string; source: string; translated: string; from: string; to: string; createdAt: string };
export type TranslationResult = { result: SavedTranslation; cached: boolean; used: number; month: string; limit: number };

const FALLBACK: TranslatorSettings = {region:'global',from:'auto',to:'zh-Hans',monthlyLimit:1_500_000,glossary:[]};
const LOCAL_SETTINGS = 'research-reader-v042-translator-settings';
const LOCAL_CACHE = 'research-reader-v042-translations';
const LOCAL_USAGE = 'research-reader-v042-usage';
let ready: Promise<void>|null=null;

function browserRead<T>(key:string,fallback:T):T {try{return JSON.parse(localStorage.getItem(key)||'null')??fallback}catch{return fallback}}
function browserWrite<T>(key:string,value:T){localStorage.setItem(key,JSON.stringify(value));}

async function ensureStore(){
  if(!isTauri())return;
  if(!ready){ready=(async()=>{
    const db=await getDatabase();
    await db.execute(`CREATE TABLE IF NOT EXISTS translation_preferences (
      id INTEGER PRIMARY KEY CHECK(id=1), region TEXT NOT NULL, source_lang TEXT NOT NULL,
      target_lang TEXT NOT NULL, monthly_limit INTEGER NOT NULL, glossary TEXT NOT NULL
    )`);
    await db.execute(`CREATE TABLE IF NOT EXISTS translation_cache (
      cache_key TEXT PRIMARY KEY, source_text TEXT NOT NULL, translated_text TEXT NOT NULL,
      source_lang TEXT NOT NULL, target_lang TEXT NOT NULL, created_at TEXT NOT NULL
    )`);
    await db.execute(`CREATE TABLE IF NOT EXISTS translation_usage (
      month TEXT PRIMARY KEY, used_chars INTEGER NOT NULL DEFAULT 0
    )`);
  })().catch(e=>{ready=null;throw e});}
  return ready;
}

function cleanSettings(s:TranslatorSettings):TranslatorSettings {
  const region=s.region.trim().toLowerCase();
  if(!/^(global|[a-z0-9-]{2,45})$/.test(region)) throw new Error('Azure 资源区域无效');
  if(!['auto','en','zh-Hans'].includes(s.from) || !['zh-Hans','en'].includes(s.to)) throw new Error('翻译语言无效');
  if(!Number.isSafeInteger(s.monthlyLimit)||s.monthlyLimit<1_000||s.monthlyLimit>2_000_000) throw new Error('本机预算必须在 1000 至 200 万字符之间');
  if(s.glossary.length>100)throw new Error('术语词典最多 100 条');
  const glossary=s.glossary.map(g=>({source:g.source.trim(),target:g.target.trim()}))
   .filter(g=>g.source&&g.target);
  if(glossary.some(g=>g.source.length>80||g.target.length>100)) throw new Error('单条术语过长');
  return {...s,region,glossary};
}

let migrated:Promise<unknown>|null=null;
async function nativeTranslation<T>(op:string,v?:unknown):Promise<T>{
 const b=electronBridge();if(!b)throw Error('Native translation store unavailable');
 if(!migrated)migrated=b.research('translation.import',{
  settings:cleanSettings(browserRead(LOCAL_SETTINGS,FALLBACK)),
  cache:Object.values(browserRead<Record<string,SavedTranslation>>(LOCAL_CACHE,{})),
  usage:browserRead<Record<string,number>>(LOCAL_USAGE,{})
 }).catch(e=>{migrated=null;throw e;});
 await migrated;return b.research(op,v) as Promise<T>;
}
export async function loadTranslatorSettings():Promise<TranslatorSettings>{
  if(electronBridge())return cleanSettings((await nativeTranslation<TranslatorSettings|null>('translation.settings'))||FALLBACK);
  if(!isTauri())return cleanSettings(browserRead(LOCAL_SETTINGS,FALLBACK));
  await ensureStore(); const db=await getDatabase();
  const rows=await db.select<Array<{region:string;source_lang:string;target_lang:string;monthly_limit:number;glossary:string}>>('SELECT * FROM translation_preferences WHERE id=1');
  if(!rows.length)return {...FALLBACK,glossary:[]};
  try{const r=rows[0];return cleanSettings({region:r.region,from:r.source_lang as TranslatorSettings['from'],to:r.target_lang as TranslatorSettings['to'],monthlyLimit:r.monthly_limit,glossary:JSON.parse(r.glossary) as GlossaryEntry[]});}
  catch{return {...FALLBACK,glossary:[]};}
}

export async function saveTranslatorSettings(input:TranslatorSettings):Promise<TranslatorSettings>{
  const s=cleanSettings(input);
  if(electronBridge())return nativeTranslation<TranslatorSettings>('translation.saveSettings',s);
  if(!isTauri()){browserWrite(LOCAL_SETTINGS,s);return s;}
  await ensureStore();const db=await getDatabase();
  await db.execute(`INSERT INTO translation_preferences(id,region,source_lang,target_lang,monthly_limit,glossary)
   VALUES(1,$1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET
   region=excluded.region,source_lang=excluded.source_lang,target_lang=excluded.target_lang,
   monthly_limit=excluded.monthly_limit,glossary=excluded.glossary`,
   [s.region,s.from,s.to,s.monthlyLimit,JSON.stringify(s.glossary)]);
  return s;
}

export async function setAzureTranslatorKey(key:string):Promise<void>{
  const bridge=electronBridge();if(bridge){await bridge.azureKey('save',key);return;}
  if(!isTauri())throw new Error('仅 Windows 桌面安装版支持安全保存 Azure Key');
  if(key.length<8||key.length>256)throw new Error('API Key 格式不正确');
  await invoke('translator_save_key',{key});
}
export async function removeAzureTranslatorKey():Promise<void>{
  const bridge=electronBridge();if(bridge){await bridge.azureKey('remove');return;}
  if(!isTauri())throw new Error('请在桌面安装版设置密钥');
  await invoke('translator_remove_key');
}
export async function hasAzureTranslatorKey():Promise<boolean>{
  const bridge=electronBridge();if(bridge)return Boolean(await bridge.azureKey('has'));
  if(!isTauri())return false;
  return invoke<boolean>('translator_has_key');
}

export function monthId(date = new Date()): string {
  // Use UTC consistently across time zones; never backdate quota reservations.
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth()+1).padStart(2,'0')}`;
}
export async function getMonthlyUsage(month=monthId()):Promise<number>{
  if(electronBridge())return nativeTranslation<number>('translation.usage',month);
  if(!isTauri())return browserRead<Record<string,number>>(LOCAL_USAGE,{})[month]||0;
  await ensureStore();const db=await getDatabase();
  const rows=await db.select<Array<{used_chars:number}>>('SELECT used_chars FROM translation_usage WHERE month=$1',[month]);
  return rows[0]?.used_chars??0;
}
async function reserveUsage(chars:number,limit:number,month:string):Promise<void>{
  if(chars===0)return;
  if(electronBridge())throw new Error('Electron quota is reserved by the main process');
  if(!isTauri()){
    const usage=browserRead<Record<string,number>>(LOCAL_USAGE,{});
    if((usage[month]||0)+chars>limit)throw new Error('超过本月本机翻译预算，请下月再试或调整预算设置');
    browserWrite(LOCAL_USAGE,{...usage,[month]:(usage[month]||0)+chars});return;
  }
  await ensureStore();const db=await getDatabase();
  await db.execute('INSERT INTO translation_usage(month,used_chars) VALUES($1,0) ON CONFLICT(month) DO NOTHING',[month]);
  const result=await db.execute('UPDATE translation_usage SET used_chars=used_chars+$1 WHERE month=$2 AND used_chars+$1<=$3',[chars,month,limit]);
  if(result.rowsAffected!==1)throw new Error('超过本月本机翻译预算，请下月再试或调整预算设置');
}
async function hashText(text:string):Promise<string>{
  const bytes=new TextEncoder().encode(text);
  const hash=await crypto.subtle.digest('SHA-256',bytes);
  return Array.from(new Uint8Array(hash),b=>b.toString(16).padStart(2,'0')).join('');
}
async function fetchCached(cacheKey:string):Promise<SavedTranslation|null>{
  if(electronBridge())return nativeTranslation<SavedTranslation|null>('translation.cache',cacheKey);
  if(!isTauri())return browserRead<Record<string,SavedTranslation>>(LOCAL_CACHE,{})[cacheKey]||null;
  await ensureStore();const db=await getDatabase();
  const rows=await db.select<Array<{cache_key:string;source_text:string;translated_text:string;source_lang:string;target_lang:string;created_at:string}>>('SELECT * FROM translation_cache WHERE cache_key=$1',[cacheKey]);
  return rows.length?{cacheKey,source:rows[0].source_text,translated:rows[0].translated_text,from:rows[0].source_lang,to:rows[0].target_lang,createdAt:rows[0].created_at}:null;
}
async function persistCached(item:SavedTranslation):Promise<void>{
  if(electronBridge()){await nativeTranslation('translation.saveCache',item);return;}
  if(!isTauri()){const old=browserRead<Record<string,SavedTranslation>>(LOCAL_CACHE,{});browserWrite(LOCAL_CACHE,{...old,[item.cacheKey]:item});return;}
  await ensureStore();const db=await getDatabase();
  await db.execute(`INSERT OR REPLACE INTO translation_cache(cache_key,source_text,translated_text,source_lang,target_lang,created_at)
   VALUES($1,$2,$3,$4,$5,$6)`,[item.cacheKey,item.source,item.translated,item.from,item.to,item.createdAt]);
}

/** Reusable by PDF, research records and later Word; user action is required. */
export async function translateResearchContent(source:string,settings:TranslatorSettings):Promise<TranslationResult>{
  const s=cleanSettings(settings),text=source.trim();
  if(!text)throw new Error('请先选择需要翻译的文字');
  if(text.length>25_000)throw new Error('单次最多翻译 2.5 万字符，请先选择段落');
  const cacheKey=await hashText(JSON.stringify([text,s.from,s.to,s.glossary]));
  const month=monthId();
  const cached=await fetchCached(cacheKey);
  if(cached)return {result:cached,cached:true,used:await getMonthlyUsage(month),month,limit:s.monthlyLimit};
  const bridge=electronBridge();
  if(!bridge&&!isTauri())throw new Error('请在 Windows 安装版中使用 Azure 翻译');
  const segments=splitResearchMarkup(text,s.glossary);
  const inputs=translationPayload(segments);
  // Azure bills Unicode text characters; reserve before each request to fail closed.
  const chars=inputs.reduce((sum,x)=>sum+Array.from(x).length,0);
  if(chars>30_000)throw new Error('本次可翻译文字超出安全上限，请缩小选区');
  if(inputs.some(x=>Array.from(x).length>10_000))throw new Error('单个段落过长，请拆分后翻译');
  if(chars>0){
    if(!await hasAzureTranslatorKey())throw new Error('请先在翻译设置中保存 Azure Translator Key');
    if(!bridge)await reserveUsage(chars,s.monthlyLimit,month);
  }
  // The provider receives only natural-language segments, not protected LaTeX or code.
  let output:string[]=[];
  if(inputs.length){
    const batches:string[][]=[];let batch:string[]=[];let total=0;
    for(const input of inputs){const len=Array.from(input).length;
      if(batch.length>=100 || total+len>10_000){batches.push(batch);batch=[];total=0;}
      batch.push(input);total+=len;
    }
    if(batch.length)batches.push(batch);
    for(const batch of batches){
      const request={texts:batch,region:s.region,from:s.from,to:s.to};
      const answer=bridge?await bridge.azureTranslate(request):await invoke<{translations:string[]}>('translator_translate',request);
      if(!Array.isArray(answer.translations)||answer.translations.length!==batch.length)throw new Error('Azure 返回的译文片段数量异常');
      output.push(...answer.translations);
    }
  }
  const result:SavedTranslation={cacheKey,source:text,translated:assembleResearchTranslation(segments,output),from:s.from,to:s.to,createdAt:new Date().toISOString()};
  await persistCached(result);
  return {result,cached:false,used:await getMonthlyUsage(month),month,limit:s.monthlyLimit};
}

export async function findSavedTranslation(source:string,settings:TranslatorSettings):Promise<SavedTranslation|null>{
  if(!source.trim())return null;
  const s=cleanSettings(settings);
  return fetchCached(await hashText(JSON.stringify([source.trim(),s.from,s.to,s.glossary])));
}
