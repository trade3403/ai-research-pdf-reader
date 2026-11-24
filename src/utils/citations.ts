import type { LiteratureItem } from './libraryStore';

export type CitationRecord = Pick<LiteratureItem,'title'|'authors'|'year'|'journal'|'doi'|'abstract'>;
export function normalizeDoi(raw:string):string {
  return raw.trim().replace(/^https?:\/\/(?:dx\.)?doi\.org\//i,'').replace(/^doi\s*:\s*/i,'').replace(/[\s.,;]+$/,'').toLowerCase();
}
export function extractDoi(text:string):string {
  const match=text.match(/10\.\d{4,9}\/[\w.()/:;+-]+/i);
  return match ? normalizeDoi(match[0]) : '';
}
export function titleFromFilename(filename:string):string {
  return filename.replace(/\.pdf$/i,'').replace(/[_]+/g,' ').replace(/\s*\(\d+\)$/,'').replace(/^\d{4}[-_\s]+/,'').trim();
}
function escapeBib(value:string):string {
  return value.replace(/\\/g,'\\textbackslash ').replace(/[{}]/g,c=>c==='{'?'\\{':'\\}').replace(/\r?\n/g,' ');
}
export function exportBibtex(items:LiteratureItem[]):string {
  const seen=new Set<string>();
  return items.map((i,index)=>{
    const lead=(i.authors.split(/[,;]|\band\b/i)[0]||'research').trim().split(/\s+/).pop()||'research';
    let key=(lead+(i.year||'')+i.title.split(/\s+/)[0]).normalize('NFKD').replace(/[^\w]/g,'').toLowerCase()||`ref${index+1}`;
    let n=2;const base=key;while(seen.has(key))key=base+n++;seen.add(key);
    const fields:[string,string][]=[['title',i.title],['author',i.authors.replace(/\s*[;,]\s*/g,' and ')],['year',i.year?String(i.year):''],['journal',i.journal],['doi',i.doi],['abstract',i.abstract]];
    return `@article{${key},\n${fields.filter(x=>x[1]).map(([k,v])=>`  ${k} = {${escapeBib(v)}}`).join(',\n')}\n}`;
  }).join('\n\n')+'\n';
}
export function exportRis(items:LiteratureItem[]):string {
  return items.map(i=>{
    const lines=['TY  - JOUR',`TI  - ${i.title}`];
    for(const author of i.authors.split(/\s*;\s*|\s+and\s+/i).map(x=>x.trim()).filter(Boolean))lines.push(`AU  - ${author}`);
    if(i.year)lines.push(`PY  - ${i.year}`);
    if(i.journal)lines.push(`JO  - ${i.journal}`);
    if(i.doi)lines.push(`DO  - ${i.doi}`);
    if(i.abstract)lines.push(`AB  - ${i.abstract.replace(/\r?\n/g,' ')}`);
    lines.push('ER  - ');return lines.join('\r\n');
  }).join('\r\n\r\n')+'\r\n';
}
export function parseRis(raw:string):CitationRecord[] {
  const result:CitationRecord[]=[];let record:Record<string,string[]>={};
  function flush(){
    if(!record.TI?.[0]&&!record.T1?.[0]){record={};return;}
    result.push({title:record.TI?.[0]||record.T1?.[0]||'',authors:(record.AU||record.A1||[]).join('; '),
      year:parseYear(record.PY?.[0]||record.Y1?.[0]||''), journal:record.JO?.[0]||record.JF?.[0]||record.T2?.[0]||'',
      doi:normalizeDoi(record.DO?.[0]||''),abstract:record.AB?.[0]||''});record={};
  }
  for(const line of raw.split(/\r?\n/)){
    const match=line.match(/^([A-Z][A-Z0-9])\s{1,2}-\s?(.*)$/);
    if(!match)continue;
    if(match[1]==='TY'&&Object.keys(record).length)flush();
    if(match[1]==='ER'){flush();continue;}
    (record[match[1]]??=[]).push(match[2].trim());
  }
  flush();return result;
}
function parseYear(s:string):number|null {
  const m=s.match(/(?:19|20)\d{2}/);return m?Number(m[0]):null;
}
/** Brace-aware BibTeX parser for the most common @article/@book records; no arbitrary evaluation. */
export function parseBibtex(source:string):CitationRecord[] {
  const records:CitationRecord[]=[];let cursor=0;
  while(cursor<source.length){
    const at=source.indexOf('@',cursor);if(at<0)break;
    const header=source.slice(at).match(/^@([a-zA-Z]+)\s*([{(])/);
    if(!header){cursor=at+1;continue;}
    const type=header[1].toLowerCase();const open=header[2];const close=open==='{'?'}':')';
    let start=at+header[0].length,depth=1,quote=false,escaped=false,end=start;
    for(;end<source.length;end++){
      const ch=source[end];if(escaped){escaped=false;continue;}if(ch==='\\'){escaped=true;continue;}
      if(ch==='"'){quote=!quote;continue;}if(quote)continue;
      if(ch===open)depth++;if(ch===close){depth--;if(depth===0)break;}
    }
    if(end>=source.length)break;const content=source.slice(start,end);cursor=end+1;
    if(['comment','preamble','string'].includes(type))continue;
    const comma=content.indexOf(',');if(comma<0)continue;
    const fields:Record<string,string>={};const part=content.slice(comma+1);
    const pattern=/([a-zA-Z][\w-]*)\s*=\s*/g;let pos=0;
    while(pos<part.length){pattern.lastIndex=pos;const m=pattern.exec(part);if(!m)break;const key=m[1].toLowerCase();let p=pattern.lastIndex;
      if(part[p]==='{'){let d=1;const startValue=++p;while(p<part.length&&d){if(part[p]==='{')d++;if(part[p]==='}')d--;p++;}fields[key]=part.slice(startValue,p-1).replace(/\\([{}])/g,'$1');}
      else if(part[p]==='"'){const startValue=++p;while(p<part.length&&(part[p]!=='"'||part[p-1]==='\\'))p++;fields[key]=part.slice(startValue,p);p++;}
      else {const startValue=p;while(p<part.length&&part[p]!==',')p++;fields[key]=part.slice(startValue,p).trim();}
      pos=p+1;
    }
    if(fields.title)records.push({title:fields.title,authors:fields.author||'',year:parseYear(fields.year||''),journal:fields.journal||fields.booktitle||'',doi:normalizeDoi(fields.doi||''),abstract:fields.abstract||''});
  }
  return records;
}
export function makeMetadataOnlyRecord(record:CitationRecord):LiteratureItem {
  const now=new Date().toISOString();return {id:crypto.randomUUID(),fingerprint:null,sha256:null,path:null,filename:'',fileSize:0,
    ...record,doi:normalizeDoi(record.doi),starred:false,addedAt:now,updatedAt:now,indexStatus:'none',tags:[],collectionIds:[]};
}
