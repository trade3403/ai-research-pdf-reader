import type { JSONContent } from '@tiptap/core';

/** Import explicitly selected research notes as editable paragraphs. */
export function researchToWordNodes(body: string, title?: string, source?: string): JSONContent[] {
  const output:JSONContent[]=[];
  if(title) output.push({type:'heading',attrs:{level:2},content:[{type:'text',text:title.slice(0,240)}]});
  let code=false;let math=false;let mathLines:string[]=[];let codeLines:string[]=[];
  const lines=body.replace(/\r\n?/g,'\n').slice(0,200_000).split('\n');
  for(const line of lines){
    if(line.trim().startsWith('```')){
      if(code){output.push({type:'codeBlock',content:[{type:'text',text:codeLines.join('\n')||' '} ]});codeLines=[];}
      code=!code;continue;
    }
    if(code){codeLines.push(line);continue;}
    const trimmed=line.trim();
    if(trimmed==='$$'||trimmed==='\\['){
      if(math){output.push({type:'wordBlockMath',attrs:{id:crypto.randomUUID(),latex:mathLines.join('\n')}});mathLines=[];}
      math=!math;continue;
    }
    if(math){mathLines.push(line);continue;}
    if(!trimmed){output.push({type:'paragraph'});continue;}
    if(/^\$\$.*\$\$$/.test(trimmed)) {output.push({type:'wordBlockMath',attrs:{id:crypto.randomUUID(),latex:trimmed.slice(2,-2)}});continue;}
    const match=/^(#{1,3})\s+(.+)$/.exec(trimmed);
    if(match){output.push({type:'heading',attrs:{level:match[1].length},content:[{type:'text',text:match[2]}]});continue;}
    output.push({type:'paragraph',content:[{type:'text',text:line}]});
  }
  if(codeLines.length) output.push({type:'codeBlock',content:[{type:'text',text:codeLines.join('\n')}]});
  if(mathLines.length) output.push({type:'wordBlockMath',attrs:{id:crypto.randomUUID(),latex:mathLines.join('\n')}});
  if(source && /^https:\/\//.test(source))output.push({type:'paragraph',content:[{type:'text',text:`来源：${source}`}]});
  return output.length?output:[{type:'paragraph'}];
}
