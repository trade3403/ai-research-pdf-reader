import type { JSONContent } from '@tiptap/core';

export type AcademicKind = 'equation' | 'figure' | 'table';
export type CitationStyle = 'numeric' | 'author-year';
export type AcademicTarget = { id: string; kind: AcademicKind; number: number; title: string; label: string };
export type CitationSnapshot = { paperId: string; title: string; authors: string; year: number | string | null; journal: string; doi: string };
export type AcademicIndex = {
  targets: AcademicTarget[];
  targetById: Map<string, AcademicTarget>;
  citations: CitationSnapshot[];
  citationNumbers: Map<string, number>;
};

const kindLabels: Record<AcademicKind, string> = { equation: 'Equation', figure: 'Figure', table: 'Table' };

function walk(root: JSONContent | undefined, fn: (node: JSONContent) => void): void {
  if (!root) return;
  fn(root);
  for (const child of root.content || []) walk(child, fn);
}
export function academicIndex(doc: JSONContent): AcademicIndex {
  const counts: Record<AcademicKind,number> = {equation:0,figure:0,table:0};
  const targets: AcademicTarget[]=[];
  const targetById=new Map<string,AcademicTarget>();
  const citations:CitationSnapshot[]=[];
  const citationNumbers=new Map<string,number>();
  walk(doc,node=>{
    const kind: AcademicKind|null = node.type==='wordBlockMath' ? 'equation' : node.type==='wordCaption' ? (node.attrs?.kind === 'table'?'table':'figure') : null;
    if (kind) {
      const id=String(node.attrs?.id||'');
      const number=++counts[kind];
      const title=String(node.attrs?.text||'');
      const target={id,kind,number,title,label:`${kindLabels[kind]} ${number}`};
      targets.push(target);
      if(id&&!targetById.has(id))targetById.set(id,target);
    }
    if(node.type==='wordCitation'){
      const paperId=String(node.attrs?.paperId||'');
      if(!paperId||citationNumbers.has(paperId))return;
      citationNumbers.set(paperId,citations.length+1);
      citations.push({paperId,title:String(node.attrs?.title||''),authors:String(node.attrs?.authors||''),
        year:node.attrs?.year||null,journal:String(node.attrs?.journal||''),doi:String(node.attrs?.doi||'')});
    }
  });
  return {targets,targetById,citations,citationNumbers};
}
export function citationText(node: JSONContent, index: AcademicIndex, style:CitationStyle):string {
  const id=String(node.attrs?.paperId||'');
  if(style==='numeric')return index.citationNumbers.has(id)?`[${index.citationNumbers.get(id)}]`:'[?]';
  const authors=String(node.attrs?.authors||'');
  const surname=(authors.split(/[;,]|\band\b/i)[0]||'Unknown').trim().split(/\s+/).slice(-1)[0];
  return `(${surname||'Unknown'}, ${node.attrs?.year||'n.d.'})`;
}
export function referenceText(c:CitationSnapshot, style:CitationStyle, number:number):string {
  const year=c.year?`(${c.year}).`:'(n.d.).';
  const segments=[c.authors||'Unknown',year,c.title,c.journal,c.doi?`https://doi.org/${c.doi}`:''].filter(Boolean);
  return `${style==='numeric'?`[${number}] `:''}${segments.join(' ')}`;
}
export function crossReferenceText(targetId:string,index:AcademicIndex):string {
  const item=index.targetById.get(targetId);
  return item?item.kind==='equation'?`Equation (${item.number})`:`${item.label}`:'[Missing reference]';
}
/** Called on imports and old drafts. Stable IDs are allocated once, never on every render. */
export function ensureAcademicIds(doc:JSONContent, makeId:()=>string=()=>crypto.randomUUID()):JSONContent {
  const seen=new Set<string>();
  const clone=(node:JSONContent):JSONContent=>{
    const next:JSONContent={...node};
    if(['wordBlockMath','wordCaption'].includes(node.type||'')){
      const attrs={...(node.attrs||{})};
      const previous=String(attrs.id||'');
      if(!previous||seen.has(previous))attrs.id=makeId();
      seen.add(String(attrs.id));next.attrs=attrs;
    }
    if(node.content)next.content=node.content.map(clone);
    return next;
  };
  return clone(doc);
}
export function refreshAcademicLabels(root:HTMLElement,doc:JSONContent,style:CitationStyle):void {
  const index=academicIndex(doc);
  for(const el of root.querySelectorAll<HTMLElement>('[data-academic-label]')){
    const role=el.dataset.academicLabel;
    const id=el.dataset.academicId||'';
    if(role==='equation') el.textContent=index.targetById.has(id)?`(${index.targetById.get(id)!.number})`:'(?)';
    else if(role==='caption') el.textContent=index.targetById.get(id)?.label||'[Missing caption]';
    else if(role==='crossref')el.textContent=crossReferenceText(id,index);
    else if(role==='citation'){
      const found=index.citations.find(c=>c.paperId===id);
      el.textContent=found?citationText({attrs:{...found}},index,style):'[?]';
    }else if(role==='bibliography')el.textContent=index.citations.length
      ?index.citations.map((c,i)=>referenceText(c,style,i+1)).join('\n'):'尚无文内引用';
  }
}
