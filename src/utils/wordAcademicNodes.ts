import { Node, mergeAttributes } from '@tiptap/core';

const captionKinds=['figure','table'];
export const WordCaption=Node.create({
  name:'wordCaption',group:'block',atom:true,selectable:true,
  addAttributes(){return {id:{default:null},kind:{default:'figure'},text:{default:''}};},
  parseHTML(){return [{tag:'div[data-word-caption]'}];},
  renderHTML({HTMLAttributes}){return ['div',mergeAttributes(HTMLAttributes,{'data-word-caption':'1'})];},
  addNodeView(){return ({node,editor,getPos})=>{
    const dom=document.createElement('div');dom.className='word-academic-caption';dom.contentEditable='false';
    const label=document.createElement('strong');label.dataset.academicLabel='caption';
    const body=document.createElement('span');dom.append(label,document.createTextNode('. '),body);
    const paint=(n:typeof node)=>{label.dataset.academicId=String(n.attrs.id||'');body.textContent=String(n.attrs.text||'');};paint(node);
    dom.addEventListener('dblclick',()=>{
      const updated=window.prompt('编辑图表题注',body.textContent||'');
      if(updated===null||updated.length>1000)return;
      const pos=typeof getPos==='function'?getPos():undefined;
      if(typeof pos==='number')editor.chain().focus().setNodeSelection(pos).updateAttributes('wordCaption',{text:updated}).run();
    });
    return {dom,update(n){if(n.type.name!=='wordCaption'||!captionKinds.includes(n.attrs.kind))return false;paint(n);return true;}};
  };},
});
export const WordCrossReference=Node.create({
  name:'wordCrossReference',group:'inline',inline:true,atom:true,selectable:true,
  addAttributes(){return {targetId:{default:''}};},
  parseHTML(){return [{tag:'span[data-word-crossref]'}];},
  renderHTML({HTMLAttributes}){return ['span',mergeAttributes(HTMLAttributes,{'data-word-crossref':'1'})];},
  addNodeView(){return ({node})=>{
    const dom=document.createElement('span');dom.className='word-crossreference';dom.contentEditable='false';
    dom.dataset.academicLabel='crossref';dom.dataset.academicId=String(node.attrs.targetId||'');dom.textContent='[reference]';
    return {dom,update(n){if(n.type.name!=='wordCrossReference')return false;dom.dataset.academicId=String(n.attrs.targetId||'');return true;}};
  };},
});
export const WordCitation=Node.create({
  name:'wordCitation',group:'inline',inline:true,atom:true,selectable:true,
  addAttributes(){return {paperId:{default:''},title:{default:''},authors:{default:''},year:{default:null},journal:{default:''},doi:{default:''}};},
  parseHTML(){return [{tag:'span[data-word-citation]'}];},
  renderHTML({HTMLAttributes}){return ['span',mergeAttributes(HTMLAttributes,{'data-word-citation':'1'})];},
  addNodeView(){return ({node})=>{
    const dom=document.createElement('span');dom.className='word-citation';dom.contentEditable='false';
    dom.dataset.academicLabel='citation';dom.dataset.academicId=String(node.attrs.paperId||'');dom.textContent='[citation]';
    return {dom,update(n){if(n.type.name!=='wordCitation')return false;dom.dataset.academicId=String(n.attrs.paperId||'');return true;}};
  };},
});
export const WordBibliography=Node.create({
  name:'wordBibliography',group:'block',atom:true,selectable:true,
  parseHTML(){return [{tag:'div[data-word-bibliography]'}];},
  renderHTML(){return ['div',{'data-word-bibliography':'1'}];},
  addNodeView(){return ()=>{
    const dom=document.createElement('div');dom.className='word-bibliography';dom.contentEditable='false';
    dom.dataset.academicLabel='bibliography';dom.textContent='尚无文内引用';
    return {dom};
  };},
});
