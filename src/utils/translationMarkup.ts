/** A renderer-independent translation contract, reusable by future DOCX editors. */
export type GlossaryEntry = { source: string; target: string };
export type TranslationSegment = { kind: 'keep' | 'translate'; value: string };

const PROTECTED = /(^|\n)(?:```|~~~)[^\n]*\n[\s\S]*?\n(?:```|~~~)[ \t]*(?=\n|$)|\$\$[\s\S]*?\$\$|\\\[[\s\S]*?\\\]|\\\([\s\S]*?\\\)|(?<!\\)\$(?!\$)(?:\\.|[^$\n])+?(?<!\\)\$|`+[^`\n]+`+|https?:\/\/[^\s<>)]*/gm;

function literalEscaped(text: string): string { return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

function splitByGlossary(text: string, glossary: GlossaryEntry[]): TranslationSegment[] {
  const rules = glossary.filter(g => g.source.trim() && g.target.trim()).sort((a,b)=>b.source.length-a.source.length);
  if (!rules.length) return [{ kind: 'translate', value: text }];
  const re = new RegExp(rules.map(g => literalEscaped(g.source)).join('|'), 'gi');
  const out: TranslationSegment[] = [];
  let pos = 0; let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    const start = m.index; const end = start + m[0].length;
    // Avoid rewriting English terms inside larger English identifiers or words.
    if ((start > 0 && /[a-z\d_]/i.test(text[start-1])) || (end < text.length && /[a-z\d_]/i.test(text[end]))) continue;
    if (start > pos) out.push({kind:'translate',value:text.slice(pos,start)});
    const rule=rules.find(g=>g.source.toLocaleLowerCase()===m![0].toLocaleLowerCase());
    out.push({kind:'keep',value:rule?.target || m[0]});
    pos=end;
  }
  if(pos < text.length) out.push({kind:'translate',value:text.slice(pos)});
  return out.length ? out : [{kind:'translate',value:text}];
}

/** Never sends formulas, inline code, fenced code, URLs or glossary replacements to the provider. */
export function splitResearchMarkup(input: string, glossary: GlossaryEntry[] = []): TranslationSegment[] {
  const result: TranslationSegment[] = [];
  let cursor=0;
  const pushProse=(chunk:string)=>{
    // Preserve source line structure and Markdown list/heading prefixes.
    for(const part of chunk.split(/(\r?\n)/)){
      if(!part) continue;
      if(/^\r?\n$/.test(part)){result.push({kind:'keep',value:part});continue;}
      const prefix=part.match(/^(\s*(?:#{1,6}\s+|[-*+]\s+|\d+\.\s+|>\s+))/)?.[0] ?? '';
      if(prefix) result.push({kind:'keep',value:prefix});
      const content=part.slice(prefix.length);
      for(const item of splitByGlossary(content,glossary)) {
        if(!item.value) continue;
        if(item.kind==='translate' && !/[^\s\p{P}\p{S}\p{N}]/u.test(item.value)) result.push({kind:'keep',value:item.value});
        else result.push(item);
      }
    }
  };
  for (const match of input.matchAll(PROTECTED)) {
    const index=match.index ?? 0;
    if(index>cursor) pushProse(input.slice(cursor,index));
    result.push({kind:'keep',value:match[0]});cursor=index+match[0].length;
  }
  if(cursor<input.length) pushProse(input.slice(cursor));
  return result;
}

export function assembleResearchTranslation(parts: TranslationSegment[], translated: string[]): string {
  let index=0;
  const text=parts.map(part=>part.kind==='translate' ? translated[index++] : part.value).join('');
  if(index!==translated.length) throw new Error('译文片段数量不一致');
  return text;
}

export function translationPayload(parts: TranslationSegment[]): string[] {
  return parts.filter(p=>p.kind==='translate').map(p=>p.value);
}
