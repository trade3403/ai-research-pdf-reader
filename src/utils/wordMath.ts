import { Node, mergeAttributes } from '@tiptap/core';
import katex from 'katex';

/** Two local-only atom nodes; latex remains source text in the editor JSON. */
export function createWordMath(name: 'wordInlineMath' | 'wordBlockMath', displayMode: boolean) {
  return Node.create({
    name,
    group: displayMode ? 'block' : 'inline',
    inline: !displayMode,
    atom: true,
    selectable: true,
    addAttributes() {return { id: { default: null, parseHTML: element => element.getAttribute('data-id'), renderHTML: attr => ({'data-id':attr.id}) }, latex: { default: String.raw`E=mc^2`, parseHTML: element => element.getAttribute('data-latex'), renderHTML: attr => ({'data-latex':attr.latex}) }};},
    parseHTML() {return [{tag: displayMode ? 'div[data-word-math="block"]' : 'span[data-word-math="inline"]'}];},
    renderHTML({HTMLAttributes}) {return [displayMode?'div':'span',mergeAttributes(HTMLAttributes,{'data-word-math':displayMode?'block':'inline',class:displayMode?'word-math-block':'word-math-inline'})];},
    addNodeView() {
      return ({node,editor,getPos}) => {
        const dom=document.createElement(displayMode?'div':'span');
        dom.className=displayMode?'word-math-block':'word-math-inline';
        dom.dataset.wordMath=displayMode?'block':'inline';
        dom.contentEditable='false';
        const formula=document.createElement('span');dom.append(formula);
        const number=document.createElement('span');
        if(displayMode){number.className='word-equation-number';number.dataset.academicLabel='equation';dom.append(number);}
        const paint=(latex:string,id:string)=>{
          try{katex.render(latex,formula,{throwOnError:false,strict:'ignore',trust:false,displayMode});}
          catch{formula.textContent=latex;}
          number.dataset.academicId=id;
          dom.title='双击编辑 LaTeX 公式';
        };
        paint(String(node.attrs.latex),String(node.attrs.id||''));
        dom.addEventListener('dblclick',()=>{
          const current=String(node.attrs.latex||'');
          const updated=window.prompt('编辑 LaTeX 公式',current);
          if(updated===null||updated.length>5000) return;
          const pos=typeof getPos==='function'?getPos():undefined;
          if(typeof pos==='number') editor.chain().focus().setNodeSelection(pos).updateAttributes(name,{latex:updated}).run();
        });
        return {dom,update(updated){if(updated.type.name!==name)return false;paint(String(updated.attrs.latex),String(updated.attrs.id||''));return true;}};
      };
    },
  });
}
