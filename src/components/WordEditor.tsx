import { useCallback, useEffect, useRef, useState } from 'react';
import { electronBridge } from '../utils/electronBridge';
import { isTauri } from '@tauri-apps/api/core';
import { EditorContent, useEditor } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import { TableKit } from '@tiptap/extension-table';
import Image from '@tiptap/extension-image';
import TextAlign from '@tiptap/extension-text-align';
import { TextStyleKit } from '@tiptap/extension-text-style';
import { AlignCenter, AlignJustify, AlignLeft, AlignRight, Bold, BookOpen, FileDown,
  FilePlus, FileText, Image as ImageIcon, Italic, List, ListOrdered, Plus, Save, Printer,
  Sigma, Table as TableIcon, Trash2, Underline, Undo2, Redo2, Type, Upload } from 'lucide-react';
import { createWordMath } from '../utils/wordMath';
import { importDocxBytes } from '../utils/wordImport';
import { exportWordDocx } from '../utils/wordExport';
import { deleteManuscript, listManuscripts, makeManuscript, saveManuscript } from '../utils/wordStore';
import type { Manuscript } from '../utils/wordStore';
import { researchToWordNodes } from '../utils/researchToWord';
import {academicIndex,ensureAcademicIds,refreshAcademicLabels} from '../utils/wordAcademic';
import {WordCaption,WordCitation,WordCrossReference,WordBibliography} from '../utils/wordAcademicNodes';
import { loadLibrary } from '../utils/libraryStore';
import type { LiteratureItem } from '../utils/libraryStore';

export type WordInsert = {id:string;body:string;title?:string;source?:string};
type Props={incoming:WordInsert|null;onConsumed:()=>void;onNotify:(message:string,tone?:'error'|'info')=>void};
const inlineMath=createWordMath('wordInlineMath',false);
const blockMath=createWordMath('wordBlockMath',true);
const extensions=[StarterKit,TableKit.configure({table:{resizable:true}}),Image.configure({allowBase64:true}),
  TextAlign.configure({types:['heading','paragraph']}),TextStyleKit,inlineMath,blockMath,WordCaption,WordCitation,WordCrossReference,WordBibliography];
const WELCOME='<h1>Untitled research manuscript</h1><p>Begin writing your paper…</p>';

type TemplateKind='general'|'ieee'|'elsevier'|'springer';
function sciTemplate(kind:TemplateKind='general'):Manuscript {
  const label={general:'General SCI',ieee:'IEEE-like',elsevier:'Elsevier-like',springer:'Springer-like'}[kind];
  const doc=makeManuscript(`${label} Manuscript`);
  const sections=kind==='ieee'?['I. Introduction','II. Related Work','III. Methods','IV. Experiments','V. Results','VI. Conclusion']:
    kind==='springer'?['1 Introduction','2 Related Work','3 Mathematical Model','4 Experiments','5 Discussion','6 Conclusion']:
    kind==='elsevier'?['1. Introduction','2. Theoretical Background','3. Methodology','4. Simulation Results','5. Discussion','6. Conclusions']:
    ['1. Introduction','2. Mathematical Model','3. Numerical Method','4. Numerical Experiments','5. Results and Discussion','6. Conclusions'];
  doc.marginMm=kind==='ieee'?20:25;
  doc.content={type:'doc',content:[
    {type:'heading',attrs:{level:1},content:[{type:'text',text:'Manuscript Title'}]},
    {type:'paragraph',content:[{type:'text',text:'Authors and affiliations'}]},
    {type:'heading',attrs:{level:2},content:[{type:'text',text:'Abstract'}]},
    {type:'paragraph',content:[{type:'text',text:'Research background, methodology, results, and contributions.'}]},
    {type:'paragraph',content:[{type:'text',text:'Keywords: dynamics; numerical stability; simulation'}]},
    ...sections.flatMap((s)=>[
      {type:'heading',attrs:{level:2},content:[{type:'text',text:s}]},
      {type:'paragraph',content:[{type:'text',text:'Write this section here.'}]},
    ]),
    {type:'heading',attrs:{level:2},content:[{type:'text',text:'References'}]},
    {type:'wordBibliography'},
  ]};
  return doc;
}
export default function WordEditor({incoming,onConsumed,onNotify}:Props) {
  const [documents,setDocuments]=useState<Manuscript[]>([]);
  const [references,setReferences]=useState<LiteratureItem[]>([]);
  const [referenceId,setReferenceId]=useState('');
  const [templateKind,setTemplateKind]=useState<TemplateKind>('general');
  const [crossReferenceId,setCrossReferenceId]=useState('');
  const [active,setActive]=useState<Manuscript|null>(null);
  const [busy,setBusy]=useState(false);
  const [saveState,setSaveState]=useState('草稿已加载');
  const [showOutline,setShowOutline]=useState(true);
  const [revision,setRevision]=useState(0);
  const [fontPx,setFontPx]=useState('16');
  const fileInput=useRef<HTMLInputElement>(null);
  const imageInput=useRef<HTMLInputElement>(null);
  const activeRef=useRef<Manuscript|null>(null);
  const suppress=useRef(false);
  const writeTimer=useRef<number|null>(null);
  const alive=useRef(true);

  const editor=useEditor({extensions,content:WELCOME,immediatelyRender:false,
    onUpdate:({editor:e})=>{
      if(suppress.current)return;
      setRevision(r=>r+1);
      setActive(current=>{
        if(!current)return current;
        const next={...current,content:e.getJSON(),updatedAt:new Date().toISOString()};activeRef.current=next;
        setSaveState('未保存的更改');return next;
      });
    },
  });
  useEffect(()=>{void loadLibrary().then(s=>setReferences(s.items)).catch(()=>{});},[]);
  const refresh=useCallback(async()=>{
    try{setDocuments(await listManuscripts());}
    catch(error){onNotify(`Word 文档读取失败：${String(error)}`,'error');}
  },[onNotify]);

  const flush=useCallback(async()=>{
    if(writeTimer.current!==null){window.clearTimeout(writeTimer.current);writeTimer.current=null;}
    const doc=activeRef.current;
    if(!doc)return;
    await saveManuscript(doc);
    if(alive.current){setSaveState('已保存到本地数据库');}
  },[]);

  const select=useCallback(async(doc:Manuscript)=>{
    try{await flush();}catch(error){onNotify(`自动保存失败：${String(error)}`,'error');return;}
    const restored={...doc,content:ensureAcademicIds(doc.content)};
    activeRef.current=restored;setActive(restored);setRevision(0);
    if(editor){suppress.current=true;editor.commands.setContent(restored.content,{emitUpdate:false});suppress.current=false;}
    setSaveState('已加载本地草稿');
  },[editor,flush,onNotify]);

  useEffect(()=>{
    alive.current=true;
    let cancelled=false;
    (async()=>{
      try{
        const list=await listManuscripts();if(cancelled)return;
        setDocuments(list);
        const first=list[0]||makeManuscript();
        if(!list.length)await saveManuscript(first);
        if(cancelled)return;
        const restored={...first,content:ensureAcademicIds(first.content)};
        activeRef.current=restored;setActive(restored);
        if(editor){suppress.current=true;editor.commands.setContent(restored.content,{emitUpdate:false});suppress.current=false;}
      }catch(error){if(!cancelled)onNotify(`Word 初始化失败：${String(error)}`,'error');}
    })();
    return()=>{cancelled=true;alive.current=false; if(writeTimer.current!==null)window.clearTimeout(writeTimer.current);void flush().catch(()=>{});};
    // Word editor is loaded once when its workspace is mounted.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  },[editor]);

  useEffect(()=>{
    if(!active||!revision)return;
    if(writeTimer.current!==null)window.clearTimeout(writeTimer.current);
    writeTimer.current=window.setTimeout(()=>{
      void flush().then(()=>refresh()).catch(error=>onNotify(`自动保存失败：${String(error)}`,'error'));
    },1100);
    return()=>{if(writeTimer.current!==null)window.clearTimeout(writeTimer.current);writeTimer.current=null;};
  },[active?.updatedAt,revision,flush,refresh,onNotify]);

  useEffect(()=>{
    if(!incoming||!editor||!activeRef.current)return;
    editor.chain().focus('end').insertContent([...researchToWordNodes(incoming.body,incoming.title,incoming.source),{type:'paragraph'}]).run();
    setSaveState('科研内容已插入，等待自动保存');
    onConsumed();
  },[incoming,editor,onConsumed,active?.id]);

  const create=async(template=false)=>{
    if(!editor)return;
    const doc=template?sciTemplate(templateKind):makeManuscript();
    try{await select(doc);await saveManuscript(doc);await refresh();onNotify(template?'已新建 SCI 论文结构':'已创建 Word 草稿');}
    catch(error){onNotify(`创建文档失败：${String(error)}`,'error');}
  };
  const updateMeta=(changes:Partial<Manuscript>)=>{
    setActive(current=>{
      if(!current)return current;
      const next={...current,...changes,updatedAt:new Date().toISOString()};activeRef.current=next;setRevision(r=>r+1);return next;
    });
  };
  const remove=async()=>{
    if(!active||!window.confirm(`确认删除本地草稿「${active.title}」？不会删除原始 DOCX 文件。`))return;
    try{if(writeTimer.current!==null){window.clearTimeout(writeTimer.current);writeTimer.current=null;}
      activeRef.current=null;await deleteManuscript(active.id);
      const next=(await listManuscripts())[0]||makeManuscript();
      if(next.id!==active.id&&!(await listManuscripts()).length)await saveManuscript(next);
      const restored={...next,content:ensureAcademicIds(next.content)};
      activeRef.current=restored;setActive(restored);
      if(editor){suppress.current=true;editor.commands.setContent(restored.content,{emitUpdate:false});suppress.current=false;}
      await refresh();onNotify('已删除本地草稿');
    }catch(error){onNotify(`删除草稿失败：${String(error)}`,'error');}
  };
  const loadBytes=async(bytes:Uint8Array,name:string,path:string|null)=>{
    if(!editor)throw new Error('编辑器尚未准备好');
    await flush();
    const converted=await importDocxBytes(bytes);
    const doc=makeManuscript(name.replace(/\.docx$/i,''));doc.sourcePath=path;
    suppress.current=true;
    try{editor.commands.setContent(converted.html||'<p></p>',{emitUpdate:false});doc.content=ensureAcademicIds(editor.getJSON());}
    finally{suppress.current=false;}
    activeRef.current=doc;setActive(doc);setRevision(0);
    await saveManuscript(doc);await refresh();
    setSaveState('DOCX 已导入本地草稿');
    if(converted.warnings.length) onNotify(`已导入，转换提示 ${converted.warnings.length} 条；复杂版式可能变化`);
    else onNotify('DOCX 已导入，可编辑并另存');
  };
  const loadFromFiles=async(files:FileList|null)=>{
    if(!files?.length)return;
    setBusy(true);
    try{const file=files[0];if(!file.name.toLowerCase().endsWith('.docx'))throw new Error('只支持 .docx 文件');
      await loadBytes(new Uint8Array(await file.arrayBuffer()),file.name,null);
    }catch(error){onNotify(`Word 导入失败：${String(error)}`,'error');}
    finally{setBusy(false);if(fileInput.current)fileInput.current.value='';}
  };
  const importDocument=async()=>{
    if(!isTauri()){fileInput.current?.click();return;}
    setBusy(true);
    try{const {open}=await import('@tauri-apps/plugin-dialog');const path=await open({multiple:false,filters:[{name:'Word 文档',extensions:['docx']}]});
      if(typeof path!=='string')return;
      const {readFile}=await import('@tauri-apps/plugin-fs');const bytes=await readFile(path);
      await loadBytes(bytes,path.split(/[\\/]/).pop()||'Imported.docx',path);
    }catch(error){onNotify(`Word 导入失败：${String(error)}`,'error');}
    finally{setBusy(false);}
  };
  const exportDocument=async()=>{
    const doc=activeRef.current;
    if(!doc||!editor)return;
    setBusy(true);
    try{await flush();const latest={...doc,content:editor.getJSON()};
      const blob=await exportWordDocx(latest);
      if(blob.size>32*1024*1024)throw new Error('DOCX 导出大于 32 MB');
      const basename=(doc.title.replace(/[\\/:*?"<>|]/g,'_').slice(0,80)||'Manuscript')+'.docx';
      const bridge=electronBridge();
      if(bridge){
        const path=await bridge.docxSave(basename,new Uint8Array(await blob.arrayBuffer()));
        if(!path)return;
        updateMeta({sourcePath:path});
      }else if(isTauri()){
        const {save}=await import('@tauri-apps/plugin-dialog');
        const path=await save({defaultPath:doc.sourcePath||basename,filters:[{name:'Word 文档',extensions:['docx']}]});
        if(!path)return;
        const {invoke}=await import('@tauri-apps/api/core');
        await invoke('write_word_docx',{path,bytes:Array.from(new Uint8Array(await blob.arrayBuffer()))});
        updateMeta({sourcePath:path});
      }else{
        const url=URL.createObjectURL(blob);const a=document.createElement('a');a.href=url;a.download=basename;a.click();
        window.setTimeout(()=>URL.revokeObjectURL(url),5000);
      }
      onNotify('Word DOCX 已导出（公式优先转换为原生 Word 公式）');
    }catch(error){onNotify(`Word 导出失败：${String(error)}`,'error');}
    finally{setBusy(false);}
  };
  const insertImage=async(files:FileList|null)=>{
    if(!editor||!files?.length)return;
    const file=files[0];
    if(file.size>1_500_000||!['image/png','image/jpeg','image/gif'].includes(file.type)){
      onNotify('图片限 PNG/JPEG/GIF，单张最大 1.5 MB','error');return;
    }
    const reader=new FileReader();reader.onload=()=>{
      const src=String(reader.result||'');editor.chain().focus().setImage({src,alt:file.name}).run();
    };reader.readAsDataURL(file);
    if(imageInput.current)imageInput.current.value='';
  };
  const formula=()=>{
    if(!editor)return;
    const latex=window.prompt('输入 LaTeX 公式（导出时转换为 Word 原生公式）',String.raw`E^{n+1}-E^n\leq 0`);
    if(!latex||latex.length>5000)return;
    editor.chain().focus().insertContent({type:'wordBlockMath',attrs:{id:crypto.randomUUID(),latex}}).run();
  };
  const insertCitation=()=>{
    const paper=references.find(item=>item.id===referenceId);if(!paper||!editor)return;
    editor.chain().focus().insertContent({type:'wordCitation',attrs:{paperId:paper.id,title:paper.title,
      authors:paper.authors,year:paper.year||null,journal:paper.journal,doi:paper.doi}}).run();
  };
  const insertBibliography=()=>{
    if(!editor)return;
    if(editor.getJSON().content?.some(n=>n.type==='wordBibliography')){
      onNotify('已有动态参考文献列表；文内引用改变时会自动更新');return;
    }
    editor.chain().focus('end').insertContent([{type:'heading',attrs:{level:2},content:[{type:'text',text:'References'}]},
      {type:'wordBibliography'},{type:'paragraph'}]).run();
  };
  const insertCaption=(kind:'figure'|'table')=>{
    if(!editor)return;
    const content=window.prompt(kind==='figure'?'图题文字':'表题文字','');
    if(content===null||!content.trim()||content.length>1000)return;
    editor.chain().focus().insertContent({type:'wordCaption',attrs:{id:crypto.randomUUID(),kind,text:content.trim()}}).run();
  };
  const insertCrossRef=()=>{
    if(!editor||!crossReferenceId)return;
    editor.chain().focus().insertContent({type:'wordCrossReference',attrs:{targetId:crossReferenceId}}).run();
  };
  const index=academicIndex(active?.content||{type:'doc',content:[]});
  useEffect(()=>{
    if(!editor)return;
    const current=activeRef.current;
    if(current)refreshAcademicLabels(editor.view.dom,current.content,current.citationStyle||'numeric');
  },[editor,revision,active?.id,active?.citationStyle]);
  const outline=(active?.content.content||[]).filter(n=>n.type==='heading').slice(0,150);
  return <div className="word-workspace">
    <input ref={fileInput} type="file" accept=".docx,application/vnd.openxmlformats-officedocument.wordprocessingml.document" className="hidden" onChange={e=>void loadFromFiles(e.target.files)} />
    <input ref={imageInput} type="file" accept="image/png,image/jpeg,image/gif" className="hidden" onChange={e=>void insertImage(e.target.files)} />
    <aside className="word-documents">
      <div className="word-sidebar-header"><strong><BookOpen size={16}/> Word 论文</strong><button title="新建空白文档" onClick={()=>void create()}><Plus size={17}/></button></div>
      <div className="word-sidebar-buttons"><select aria-label="SCI 论文结构模板" title="仅提供结构预设，不代表出版商官方 Word 模板" value={templateKind} onChange={e=>setTemplateKind(e.target.value as TemplateKind)}><option value="general">通用 SCI 结构</option><option value="ieee">IEEE 类结构</option><option value="elsevier">Elsevier 类结构</option><option value="springer">Springer 类结构</option></select><button onClick={()=>void create(true)}><FilePlus size={15}/> 新建模板</button><button onClick={()=>void importDocument()}><Upload size={15}/> 打开 DOCX</button></div>
      <div className="word-document-list">{documents.map(doc=><button key={doc.id} className={active?.id===doc.id?'selected':''} onClick={()=>void select(doc)}><FileText size={15}/><span><strong>{doc.title}</strong><small>{new Date(doc.updatedAt).toLocaleString()}</small></span></button>)}</div>
      {showOutline&&<div className="word-outline"><strong>本文标题大纲</strong>{outline.map((node,i)=><div key={i} style={{paddingLeft:Math.min(3,Number(node.attrs?.level||1))*8}}>{node.content?.map(c=>c.text||'').join('')}</div>)}</div>}
    </aside>
    <main className="word-main">
      <header className="word-editor-header"><div className="word-title-line"><Type size={18}/><input aria-label="Word 文档标题" value={active?.title||''} onChange={e=>updateMeta({title:e.target.value})} maxLength={240}/><span>{saveState}</span></div>
        <div className="word-file-actions"><button className="button" disabled={busy} onClick={()=>void flush().then(refresh)}><Save size={15}/> 保存草稿</button><button className="button" disabled={!active} title="打开系统打印对话框，选择保存为 PDF；并非独立 PDF 转换引擎" onClick={()=>window.print()}><Printer size={15}/> 打印 / PDF</button><button className="button primary" disabled={busy||!active} onClick={()=>void exportDocument()}><FileDown size={15}/> 导出 DOCX</button><button className="icon-button" title="删除草稿" onClick={()=>void remove()}><Trash2 size={15}/></button></div>
      </header>
      <div className="word-toolbar" role="toolbar" aria-label="Word 论文编辑工具栏">
        <select title="段落样式" defaultValue="paragraph" onChange={e=>{if(!editor)return;const v=e.target.value;if(v==='paragraph')editor.chain().focus().setParagraph().run();else editor.chain().focus().setHeading({level:Number(v) as 1|2|3}).run();}}>
          <option value="paragraph">正文</option><option value="1">一级标题</option><option value="2">二级标题</option><option value="3">三级标题</option>
        </select>
        <select title="字体" defaultValue="Times New Roman" onChange={e=>editor?.chain().focus().setFontFamily(e.target.value).run()}><option>Times New Roman</option><option>Arial</option><option>Calibri</option><option>宋体</option></select>
        <select title="字号（px）" value={fontPx} onChange={e=>{setFontPx(e.target.value);editor?.chain().focus().setFontSize(`${e.target.value}px`).run();}}><option value="12">9pt</option><option value="14">10.5pt</option><option value="16">12pt</option><option value="20">15pt</option><option value="24">18pt</option><option value="32">24pt</option></select>
        <span className="word-toolbar-separator"/>
        <button title="加粗" className={editor?.isActive('bold')?'active':''} onClick={()=>editor?.chain().focus().toggleBold().run()}><Bold size={16}/></button>
        <button title="斜体" onClick={()=>editor?.chain().focus().toggleItalic().run()}><Italic size={16}/></button>
        <button title="下划线" onClick={()=>editor?.chain().focus().toggleUnderline().run()}><Underline size={16}/></button>
        <span className="word-toolbar-separator"/>
        <button title="左对齐" onClick={()=>editor?.chain().focus().setTextAlign('left').run()}><AlignLeft size={16}/></button>
        <button title="居中" onClick={()=>editor?.chain().focus().setTextAlign('center').run()}><AlignCenter size={16}/></button>
        <button title="右对齐" onClick={()=>editor?.chain().focus().setTextAlign('right').run()}><AlignRight size={16}/></button>
        <button title="两端对齐" onClick={()=>editor?.chain().focus().setTextAlign('justify').run()}><AlignJustify size={16}/></button>
        <button title="项目列表" onClick={()=>editor?.chain().focus().toggleBulletList().run()}><List size={16}/></button>
        <button title="编号列表" onClick={()=>editor?.chain().focus().toggleOrderedList().run()}><ListOrdered size={16}/></button>
        <span className="word-toolbar-separator"/>
        <button title="插入表格" onClick={()=>editor?.chain().focus().insertTable({rows:3,cols:3,withHeaderRow:true}).run()}><TableIcon size={16}/></button>
        <button title="添加表格行" disabled={!editor?.isActive('table')} onClick={()=>editor?.chain().focus().addRowAfter().run()}>+行</button>
        <button title="插入本地图片" onClick={()=>imageInput.current?.click()}><ImageIcon size={16}/></button>
        <button title="插入 LaTeX 公式（自动编号）" onClick={formula}><Sigma size={17}/></button>
        <button title="插入图片题注" onClick={()=>insertCaption('figure')}>图题</button>
        <button title="插入表格题注" onClick={()=>insertCaption('table')}>表题</button>
        <button title="撤销" onClick={()=>editor?.chain().focus().undo().run()}><Undo2 size={16}/></button>
        <button title="重做" onClick={()=>editor?.chain().focus().redo().run()}><Redo2 size={16}/></button>
      </div>
      <div className="word-layout-strip"><label>纸张 <select value={active?.pageSize||'A4'} onChange={e=>updateMeta({pageSize:e.target.value as 'A4'|'Letter'})}><option>A4</option><option>Letter</option></select></label>
        <label>页边距 <select value={active?.marginMm||25} onChange={e=>updateMeta({marginMm:Number(e.target.value)})}><option value={20}>20 mm</option><option value={25}>25 mm</option><option value={30}>30 mm</option></select></label>
        <label>行距 <select value={active?.lineSpacing||1.5} onChange={e=>updateMeta({lineSpacing:Number(e.target.value)})}><option value={1}>1.0</option><option value={1.15}>1.15</option><option value={1.5}>1.5</option><option value={2}>2.0</option></select></label>
        <label>引用样式 <select value={active?.citationStyle||'numeric'} onChange={e=>updateMeta({citationStyle:e.target.value as 'numeric'|'author-year'})}><option value="numeric">编号 [1]</option><option value="author-year">作者年份</option></select></label>
        <button title="设置页眉内容" onClick={()=>{const v=window.prompt('页眉文字（最多 150 字符）',active?.headerText||'');if(v!==null)updateMeta({headerText:v.slice(0,150)});}}>页眉</button>
        <button title="设置页脚内容（导出时自动添加页码）" onClick={()=>{const v=window.prompt('页脚文字（自动追加页码）',active?.footerText||'');if(v!==null)updateMeta({footerText:v.slice(0,150)});}}>页脚/页码</button>
        <button onClick={()=>setShowOutline(x=>!x)}>{showOutline?'隐藏':'显示'}大纲</button>
        <select aria-label="选择文献库中的引用" value={referenceId} onChange={e=>setReferenceId(e.target.value)}><option value="">选择引用文献…</option>{references.map(p=><option key={p.id} value={p.id}>{p.title.slice(0,60)}</option>)}</select>
        <button disabled={!referenceId} onClick={insertCitation}>插入文内引用</button><button onClick={insertBibliography}>生成参考文献列表</button>
        <select aria-label="选择要交叉引用的公式图表" value={crossReferenceId} onChange={e=>setCrossReferenceId(e.target.value)}><option value="">选择公式/图表…</option>{index.targets.map(target=><option key={target.id} value={target.id}>{target.label} {target.title.slice(0,36)}</option>)}</select>
        <button disabled={!crossReferenceId||!index.targetById.has(crossReferenceId)} onClick={insertCrossRef}>插入交叉引用</button>
        <small>连续排版预览；导出 DOCX 后由 Word 分页</small>
      </div>
      <div className="word-document-scroll"><section className={`word-page ${active?.pageSize==='Letter'?'letter':''}`}
        style={{padding:`${active?.marginMm||25}mm`,lineHeight:active?.lineSpacing||1.5}}><EditorContent editor={editor}/></section></div>
      <footer className="word-statusbar">公式/题注自动编号 · 文献与交叉引用导出时重新解析 · 本地草稿自动保存 · {editor?.getText().length||0} 字符 · 双击公式编辑 LaTeX</footer>
    </main>
  </div>;
}
