import type { JSONContent } from '@tiptap/core';
import type { Manuscript } from './wordStore';
import {academicIndex,citationText,crossReferenceText,referenceText} from './wordAcademic';
import type {AcademicIndex,CitationStyle} from './wordAcademic';
import {
  AlignmentType, Document, HeadingLevel, Header, Footer, PageNumber, ImageRun, Packer, Paragraph,
  Table, TableCell, TableRow, TextRun, WidthType,
  type ParagraphChild, type FileChild,
} from 'docx';
import { Math as WordMath, latexToMath } from 'docx/math';

/** Convert our local editor JSON to native, editable Office Open XML.
 * LaTeX equations are converted to Word's native OMML by docx/math. */
function asMath(latex:string):ParagraphChild {
  try {return new WordMath({children:latexToMath(latex)});}
  catch{return new TextRun({text:`[LaTeX: ${latex}]`,italics:true});}
}
function asImage(src:string):ImageRun|null {
  const match=/^data:image\/(png|jpeg|jpg|gif);base64,([A-Za-z0-9+/=]+)$/.exec(src);
  if(!match || src.length>4_000_000) return null;
  const raw=atob(match[2]);const data=new Uint8Array(raw.length);
  for(let i=0;i<raw.length;i++)data[i]=raw.charCodeAt(i);
  const type=match[1]==='jpg'?'jpg':match[1];
  return new ImageRun({type:type as 'png'|'jpg'|'gif',data,transformation:{width:400,height:250}});
}
type ExportContext={index:AcademicIndex;style:CitationStyle;lineSpacing:number};
function asChildren(nodes:JSONContent[]=[],context:ExportContext):ParagraphChild[] {
  return nodes.flatMap((node):ParagraphChild[]=>{
    if(node.type==='wordCitation')return [new TextRun(citationText(node,context.index,context.style))];
    if(node.type==='wordCrossReference')return [new TextRun(crossReferenceText(String(node.attrs?.targetId||''),context.index))];
    if(node.type==='wordInlineMath'||node.type==='wordBlockMath')return [asMath(String(node.attrs?.latex||''))];
    if(node.type==='image'){const img=asImage(String(node.attrs?.src||''));return img?[img]:[];}
    if(node.type==='hardBreak')return [new TextRun({break:1})];
    if(node.type==='text') {
      const marks=node.marks||[];const m=(type:string)=>marks.find(mark=>mark.type===type);
      const style=m('textStyle')?.attrs||{};
      const size=parseFloat(String(style.fontSize||'16'));
      const text=node.text||'';
      const link=m('link')?.attrs?.href;
      const body=link&&/^https:\/\//i.test(String(link))?`${text} (${link})`:text;
      return [new TextRun({text:body,bold:!!m('bold'),italics:!!m('italic'),strike:!!m('strike'),
        underline:m('underline')?{}:undefined,
        font:typeof style.fontFamily==='string'?style.fontFamily:'Times New Roman',
        size:Number.isFinite(size)?Math.min(96,Math.max(12,Math.round(size*1.5))):24,
        color:typeof style.color==='string'&&/^#[0-9a-f]{6}$/i.test(style.color)?style.color.slice(1):undefined,
      })];
    }
    return asChildren(node.content,context);
  });
}
function align(node:JSONContent) {
  const value=String(node.attrs?.textAlign||'left');
  return value==='center'?AlignmentType.CENTER:value==='right'?AlignmentType.RIGHT:value==='justify'?AlignmentType.JUSTIFIED:AlignmentType.LEFT;
}
function paragraph(node:JSONContent,context:ExportContext,bullet=false,numbered=false):Paragraph {
  const level=Number(node.attrs?.level||1);
  const heading=node.type==='heading'?[HeadingLevel.HEADING_1,HeadingLevel.HEADING_2,HeadingLevel.HEADING_3][Math.min(2,Math.max(0,level-1))]:undefined;
  return new Paragraph({children:asChildren(node.content,context),heading,alignment:align(node),
    bullet:bullet?{level:0}:undefined,
    numbering:numbered?{reference:'research-ordered',level:0}:undefined,
    spacing:{after:heading?160:120,line:Math.round(240*context.lineSpacing)},
  });
}
function rowsForTable(node:JSONContent,context:ExportContext):Table {
  const rows=(node.content||[]).filter(row=>row.type==='tableRow').map(row=>new TableRow({
    children:(row.content||[]).map(cell=>new TableCell({
      children:(cell.content||[]).map(p=>paragraph(p,context)),
      columnSpan:Math.max(1,Number(cell.attrs?.colspan||1)),
      rowSpan:Math.max(1,Number(cell.attrs?.rowspan||1)),
    }))
  }));
  return new Table({rows:rows.length?rows:[new TableRow({children:[new TableCell({children:[new Paragraph('')]})]})],width:{size:100,type:WidthType.PERCENTAGE}});
}
function blocks(nodes:JSONContent[]=[],context:ExportContext):FileChild[] {
  const output:FileChild[]=[];
  for(const node of nodes){
    if(node.type==='table'){output.push(rowsForTable(node,context));continue;}
    if(node.type==='orderedList'||node.type==='bulletList'){
      for(const li of node.content||[]) for(const p of li.content||[]) output.push(paragraph(p,context,node.type==='bulletList',node.type==='orderedList'));
      continue;
    }
    if(node.type==='wordBlockMath'){
      const number=context.index.targetById.get(String(node.attrs?.id||''))?.number;
      output.push(new Paragraph({children:[asMath(String(node.attrs?.latex||'')),new TextRun({text:`    (${number||'?'})`})],alignment:AlignmentType.CENTER,spacing:{before:180,after:180}}));
      continue;
    }
    if(node.type==='wordCaption'){
      const target=context.index.targetById.get(String(node.attrs?.id||''));
      output.push(new Paragraph({children:[new TextRun({text:`${target?.label||'[Caption]'}. ${String(node.attrs?.text||'')}`,bold:true})],alignment:AlignmentType.CENTER,spacing:{before:100,after:140}}));
      continue;
    }
    if(node.type==='wordBibliography'){
      context.index.citations.forEach((citation,i)=>output.push(new Paragraph({children:[new TextRun(referenceText(citation,context.style,i+1))],spacing:{after:100}})));
      if(!context.index.citations.length)output.push(new Paragraph('No cited references.'));
      continue;
    }
    if(node.type==='image'){output.push(new Paragraph({children:asChildren([node],context),alignment:AlignmentType.CENTER}));continue;}
    if(node.type==='horizontalRule'){output.push(new Paragraph({text:'________________________________________________',spacing:{after:120}}));continue;}
    if(node.type==='blockquote'){
      for(const item of node.content||[])output.push(new Paragraph({children:asChildren(item.content,context),indent:{left:500},spacing:{after:130}}));
      continue;
    }
    if(node.type==='codeBlock'){
      output.push(new Paragraph({children:[new TextRun({text:(node.content||[]).map(c=>c.text||'').join(''),font:'Consolas',size:18})],shading:{fill:'F1F5F9'}}));
      continue;
    }
    output.push(paragraph(node,context));
  }
  return output.length?output:[new Paragraph('')];
}
export async function exportWordDocx(manuscript:Manuscript):Promise<Blob> {
  const mm=manuscript.marginMm;
  const context:ExportContext={index:academicIndex(manuscript.content),style:manuscript.citationStyle||'numeric',lineSpacing:manuscript.lineSpacing||1.5};
  const page=manuscript.pageSize==='Letter'?{width:12240,height:15840}:{width:11906,height:16838};
  const doc=new Document({title:manuscript.title,creator:'AI Research Scientist Studio',
    numbering:{config:[{reference:'research-ordered',levels:[{level:0,format:'decimal',text:'%1.',alignment:AlignmentType.START}]}]},
    sections:[{properties:{page:{size:page,margin:{top:Math.round(mm*56.69),bottom:Math.round(mm*56.69),left:Math.round(mm*56.69),right:Math.round(mm*56.69)}}},
      headers:{default:new Header({children:[new Paragraph({text:(manuscript.headerText||'').slice(0,150),alignment:AlignmentType.CENTER})]})},
      footers:{default:new Footer({children:[new Paragraph({children:[new TextRun({children:[manuscript.footerText?`${manuscript.footerText}  ·  Page `:'Page ',PageNumber.CURRENT]})],alignment:AlignmentType.CENTER})]})},
      children:blocks(manuscript.content.content,context)}],
  });
  return Packer.toBlob(doc);
}
