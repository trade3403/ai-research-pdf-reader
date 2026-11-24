import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { isTauri, invoke } from '@tauri-apps/api/core';
import {
  ArrowDownToLine, BookOpen, Check, Database, FilePlus2, FileSearch, FileText,
  Folder, FolderPlus, ListFilter, LoaderCircle, Pencil, RefreshCw, Search, Star, Tag,
  Trash2,
} from 'lucide-react';
import { openPdf, pageText } from '../utils/pdf';
import type { Collection, LiteratureItem, LibrarySnapshot } from '../utils/libraryStore';
import { clearIndexedPages, createCollection, deleteLibraryItem, importLibraryItem,
  loadLibrary, removeCollection, saveIndexedPage, saveLibraryItem, searchIndexedItems } from '../utils/libraryStore';
import { exportBibtex, exportRis, extractDoi, makeMetadataOnlyRecord,
  normalizeDoi, parseBibtex, parseRis, titleFromFilename } from '../utils/citations';

type InspectedPdf = { path: string; filename: string; sha256: string; sizeBytes: number };
type Props = { onOpenPdf: (path:string)=>Promise<void>; onNotify:(message:string,tone?:'error'|'info')=>void };
type ImportProgress = {done:number,total:number,imported:number,skipped:number,failed:number,phase:string};
const MAX_INDEXED_TEXT=8*1024*1024;
const freshState:LibrarySnapshot={items:[],collections:[]};
function stringifyError(error:unknown) { return error instanceof Error?error.message:String(error); }
function fileSize(bytes:number) { return bytes<1024*1024?`${(bytes/1024).toFixed(0)} KB`:`${(bytes/1024/1024).toFixed(1)} MB`; }
function cleanMetadata(text:string) { return text.replace(/\0/g,'').trim().slice(0,20000); }
function byParent(collections:Collection[],parentId:string|null=null,depth=0,visited=new Set<string>()):Array<{item:Collection,depth:number}> {
  const result:Array<{item:Collection,depth:number}>=[];
  for(const item of collections.filter(x=>x.parentId===parentId).sort((a,b)=>a.name.localeCompare(b.name))){
    if(visited.has(item.id))continue;const next=new Set(visited);next.add(item.id);
    result.push({item,depth},...byParent(collections,item.id,Math.min(depth+1,8),next));
  }
  return result;
}
export default function LiteratureLibrary({onOpenPdf,onNotify}:Props) {
  const [snapshot,setSnapshot]=useState<LibrarySnapshot>(freshState);
  const [loading,setLoading]=useState(true);
  const [selectedId,setSelectedId]=useState<string|null>(null);
  const [scope,setScope]=useState('all');
  const [query,setQuery]=useState('');
  const [searchFullText,setSearchFullText]=useState(false);
  const [contentHits,setContentHits]=useState<Set<string>>(new Set());
  const [progress,setProgress]=useState<ImportProgress|null>(null);
  const [errors,setErrors]=useState<string[]>([]);
  const [editing,setEditing]=useState(false);
  const [draft,setDraft]=useState<LiteratureItem|null>(null);
  const cancelRef=useRef(false);
  const runningRef=useRef(false);
  const indexRef=useRef(false);
  const citationInput=useRef<HTMLInputElement>(null);
  const selected=useMemo(()=>snapshot.items.find(x=>x.id===selectedId)||null,[snapshot,selectedId]);
  const reload=useCallback(async()=>{
    try { const data=await loadLibrary();setSnapshot(data); }
    catch(error) {onNotify(`文献库读取失败：${stringifyError(error)}`,'error');}
    finally {setLoading(false);}
  },[onNotify]);
  useEffect(()=>{void reload();},[reload]);
  useEffect(()=>{setDraft(selected?{...selected,tags:[...selected.tags],collectionIds:[...selected.collectionIds]}:null);setEditing(false);},[selectedId,selected?.updatedAt]);
  useEffect(()=>{
    let cancelled=false;
    if(!searchFullText||!query.trim()) {setContentHits(new Set());return;}
    void searchIndexedItems(query).then(ids=>{if(!cancelled)setContentHits(ids);}).catch(error=>onNotify(`全文检索失败：${stringifyError(error)}`,'error'));
    return()=>{cancelled=true};
  },[query,searchFullText,snapshot.items,onNotify]);
  const filtered=useMemo(()=>{
    const q=query.trim().toLocaleLowerCase();
    return snapshot.items.filter(item=>{
      if(scope==='starred'&&!item.starred)return false;
      if(scope.startsWith('collection:')&&!item.collectionIds.includes(scope.slice(11)))return false;
      if(scope.startsWith('tag:')&&!item.tags.includes(scope.slice(4)))return false;
      if(!q)return true;
      const meta=[item.title,item.filename,item.authors,item.journal,item.doi,item.abstract,item.year||'',...item.tags].join(' ').toLocaleLowerCase();
      return meta.includes(q)||(searchFullText&&contentHits.has(item.id));
    }).sort((a,b)=>b.updatedAt.localeCompare(a.updatedAt));
  },[snapshot,scope,query,searchFullText,contentHits]);
  const folderTree=useMemo(()=>byParent(snapshot.collections),[snapshot.collections]);
  const allTags=useMemo(()=>Array.from(new Set(snapshot.items.flatMap(i=>i.tags))).sort(),[snapshot.items]);
  const updateSelected=useCallback(async(changes:Partial<LiteratureItem>)=>{
    if(!selected)return;
    const updated={...selected,...changes};
    try{await saveLibraryItem(updated);setSnapshot(s=>({...s,items:s.items.map(i=>i.id===updated.id?{...updated,updatedAt:new Date().toISOString()}:i)}));}
    catch(error){onNotify(`保存失败：${stringifyError(error)}`,'error');}
  },[selected,onNotify]);

  const runImport=useCallback(async(paths:string[])=>{
    if(runningRef.current||!paths.length)return;
    runningRef.current=true;cancelRef.current=false;setErrors([]);
    let imported=0,skipped=0,failed=0;
    const existing=await loadLibrary();
    const hashes=new Set(existing.items.map(i=>i.sha256).filter(Boolean));
    const knownPaths=new Set(existing.items.map(i=>i.path).filter(Boolean));
    setProgress({done:0,total:paths.length,imported,skipped,failed,phase:'导入 PDF'});
    try {
      for(let index=0;index<paths.length;index++){
        if(cancelRef.current)break;
        try {
          const info=await invoke<InspectedPdf>('inspect_pdf_file',{path:paths[index]});
          if(hashes.has(info.sha256)||knownPaths.has(info.path)){skipped++;}
          else {
            const now=new Date().toISOString();
            const item:LiteratureItem={id:crypto.randomUUID(),path:info.path,sha256:info.sha256,fingerprint:null,filename:info.filename,
              fileSize:info.sizeBytes,title:titleFromFilename(info.filename),authors:'',year:null,journal:'',doi:extractDoi(info.filename),abstract:'',
              starred:false,addedAt:now,updatedAt:now,indexStatus:'none',tags:[],collectionIds:scope.startsWith('collection:')?[scope.slice(11)]:[]};
            const result=await importLibraryItem(item);
            if(result==='inserted'){
              if(item.collectionIds.length)await saveLibraryItem(item);
              imported++;hashes.add(info.sha256);knownPaths.add(info.path);
            } else skipped++;
          }
        } catch(error){failed++;setErrors(old=>[...old.slice(-99),`${paths[index]} — ${stringifyError(error)}`]);}
        if(index%4===0||index===paths.length-1){setProgress({done:index+1,total:paths.length,imported,skipped,failed,phase:'导入 PDF'});}
      }
    } finally {
      runningRef.current=false;await reload();
      setProgress(null);onNotify(`导入完成：新增 ${imported}，跳过重复 ${skipped}，失败 ${failed}${cancelRef.current?'（已取消）':''}`);
    }
  },[scope,reload,onNotify]);
  const importFiles=useCallback(async(directory:boolean)=>{
    if(!isTauri()){onNotify('请在 Windows 桌面版中批量导入本地 PDF','error');return;}
    try {
      const {open}=await import('@tauri-apps/plugin-dialog');
      const paths=await open(directory?{directory:true,multiple:false}:{directory:false,multiple:true,filters:[{name:'PDF',extensions:['pdf']}]});
      if(!paths)return;
      if(directory){const files=await invoke<string[]>('list_pdf_paths',{root:paths});await runImport(files);}
      else await runImport(Array.isArray(paths)?paths:[paths]);
    }catch(error){onNotify(`导入失败：${stringifyError(error)}`,'error');}
  },[runImport,onNotify]);

  const createFolder=useCallback(async()=>{
    const name=window.prompt('新分类名称：');if(!name?.trim())return;
    try{
      const parentId=scope.startsWith('collection:')?scope.slice(11):null;
      const created=await createCollection(name,parentId);
      await reload();setScope('collection:'+created.id);
    }catch(error){onNotify(`创建分类失败：${stringifyError(error)}`,'error');}
  },[scope,reload,onNotify]);
  const removeFolder=useCallback(async()=>{
    if(!scope.startsWith('collection:'))return;
    const id=scope.slice(11);const folder=snapshot.collections.find(c=>c.id===id);
    if(!folder||!window.confirm(`删除分类「${folder.name}」？文献和 PDF 原文件不会被删除。`))return;
    try{await removeCollection(id);setScope('all');await reload();}catch(error){onNotify(`删除分类失败：${stringifyError(error)}`,'error');}
  },[scope,snapshot,reload,onNotify]);
  const deleteSelected=useCallback(async()=>{
    if(!selected||!window.confirm(`从文献库移除「${selected.title}」？不会删除 PDF 文件，也不会删除批注。`))return;
    try{await deleteLibraryItem(selected.id);setSelectedId(null);await reload();}catch(error){onNotify(`移除失败：${stringifyError(error)}`,'error');}
  },[selected,reload,onNotify]);
  const saveDraft=useCallback(async()=>{
    if(!draft)return;
    if(!draft.title.trim()){onNotify('论文标题不能为空','error');return;}
    try {await saveLibraryItem({...draft,title:cleanMetadata(draft.title),authors:cleanMetadata(draft.authors),journal:cleanMetadata(draft.journal),abstract:cleanMetadata(draft.abstract),doi:normalizeDoi(draft.doi)});await reload();setEditing(false);onNotify('文献资料已保存');}
    catch(error){onNotify(`元数据保存失败：${stringifyError(error)}`,'error');}
  },[draft,reload,onNotify]);
  const inspectMetadata=useCallback(async()=>{
    if(!selected?.path)return;
    try{
      const bytes=await invoke<number[]>('read_recent_pdf',{path:selected.path});
      const pdf=await openPdf(new Uint8Array(bytes));
      try {
        const metadata=await pdf.getMetadata();
        const info=metadata.info as unknown as Record<string,unknown>;
        const first=await pageText(pdf,1);
        const doi=extractDoi([selected.doi,first].join(' '));
        const title=typeof info.Title==='string'&&info.Title.trim()?info.Title:selected.title;
        const authors=typeof info.Author==='string'&&info.Author.trim()?info.Author:selected.authors;
        const year=selected.year||null;
        setDraft(prev=>prev?{...prev,title,authors,year,doi:doi||prev.doi}:null);
        setEditing(true);onNotify('已提取 PDF 可用元数据，请核实并保存');
      } finally {await pdf.cleanup();}
    }catch(error){onNotify(`PDF 元数据识别失败：${stringifyError(error)}`,'error');}
  },[selected,onNotify]);
  const onlineLookup=useCallback(async()=>{
    const doi=normalizeDoi(draft?.doi||selected?.doi||'');
    if(!doi){onNotify('请先填写 DOI','error');return;}
    if(!window.confirm('将此 DOI 发送至 Crossref 查询公开文献元数据。是否继续？'))return;
    try{
      const response=await fetch('https://api.crossref.org/works/'+encodeURIComponent(doi),{headers:{Accept:'application/json'}});
      if(!response.ok)throw new Error(`Crossref HTTP ${response.status}`);
      const data=await response.json() as {message?:{title?:string[];author?:Array<{given?:string;family?:string}>;container_title?:string[];
        'container-title'?:string[];published?:{'date-parts'?:number[][]};issued?:{'date-parts'?:number[][]};abstract?:string}};
      const m=data.message;if(!m)throw new Error('元数据为空');
      const authors=(m.author||[]).map(x=>[x.given,x.family].filter(Boolean).join(' ')).join('; ');
      const year=m.published?.['date-parts']?.[0]?.[0]||m.issued?.['date-parts']?.[0]?.[0]||null;
      setDraft(prev=>prev?{...prev,doi,title:m.title?.[0]||prev.title,authors:authors||prev.authors,
        year:year||prev.year,journal:m['container-title']?.[0]||prev.journal,
        abstract:m.abstract?m.abstract.replace(/<[^>]+>/g,' ').trim():prev.abstract}:null);
      setEditing(true);onNotify('Crossref 结果已载入，请检查并保存');
    }catch(error){onNotify(`DOI 查询失败：${stringifyError(error)}`,'error');}
  },[draft,selected,onNotify]);

  const indexSelected=useCallback(async()=>{
    if(!selected?.path||indexRef.current)return;
    indexRef.current=true;cancelRef.current=false;
    let indexed=0;let total=0;let clipped=false;
    setProgress({done:0,total:1,imported:0,skipped:0,failed:0,phase:'全文索引'});
    try{
      const bytes=await invoke<number[]>('read_recent_pdf',{path:selected.path});
      const pdf=await openPdf(new Uint8Array(bytes));
      try {
        total=pdf.numPages;await clearIndexedPages(selected.id);
        let characters=0;
        for(let page=1;page<=total;page++){
          if(cancelRef.current)break;
          const text=await pageText(pdf,page);
          if(characters+text.length>MAX_INDEXED_TEXT){clipped=true;break;}
          await saveIndexedPage(selected.id,page,text);
          indexed=page;characters+=text.length;
          if(page%3===0||page===total) setProgress({done:page,total,imported:page,skipped:0,failed:0,phase:'全文索引'});
        }
      } finally {await pdf.cleanup();}
      const status=(!cancelRef.current&&!clipped&&indexed===total)?'complete':'partial';
      await saveLibraryItem({...selected,indexStatus:status});await reload();
      onNotify(`已索引 ${indexed}/${total} 页${status==='partial'?'（部分索引）':''}`);
    }catch(error){
      try{await saveLibraryItem({...selected,indexStatus:'partial'});await reload();}catch{/* keep original error */}
      onNotify(`全文索引失败：${stringifyError(error)}`,'error');
    } finally {indexRef.current=false;setProgress(null);}
  },[selected,onNotify,reload]);

  const writeExport=useCallback(async(file:string,content:string,ext:'bib'|'ris'|'json')=>{
    if(!isTauri()){
      const url=URL.createObjectURL(new Blob([content],{type:'text/plain;charset=utf-8'}));
      const a=document.createElement('a');a.href=url;a.download=file;a.click();setTimeout(()=>URL.revokeObjectURL(url),2000);return;
    }
    const {save}=await import('@tauri-apps/plugin-dialog');
    const path=await save({defaultPath:file,filters:[{name:ext.toUpperCase(),extensions:[ext]}]});if(!path)return;
    await invoke('write_library_export',{path,content});
  },[]);
  const exportItems=useCallback(async(format:'bib'|'ris'|'json')=>{
    try{
      const items=filtered;
      const text=format==='bib'?exportBibtex(items):format==='ris'?exportRis(items):JSON.stringify({format:'research-reader-v03-metadata',note:'仅备份文献元数据，未包含 PDF 文件和原有批注',items:snapshot.items,collections:snapshot.collections},null,2);
      await writeExport(`research-library.${format}`,text,format);onNotify(`已处理 ${items.length} 条文献的 ${format.toUpperCase()} 导出`);
    }catch(error){onNotify(`导出失败：${stringifyError(error)}`,'error');}
  },[filtered,snapshot,writeExport,onNotify]);
  const importCitations=useCallback(async(file:File|null)=>{
    try{
      let content='';let ext='';
      if(file){content=await file.text();ext=file.name.split('.').pop()?.toLowerCase()||'';}
      else {
        const {open}=await import('@tauri-apps/plugin-dialog');
        const path=await open({directory:false,multiple:false,filters:[{name:'BibTeX / RIS',extensions:['bib','ris']}]});if(!path)return;
        content=await invoke<string>('read_citation_import',{path});ext=path.split('.').pop()?.toLowerCase()||'';
      }
      const records=ext==='bib'?parseBibtex(content):ext==='ris'?parseRis(content):[];
      if(!records.length)throw new Error('未识别到有效条目');
      const known=new Set(snapshot.items.filter(i=>i.doi).map(i=>normalizeDoi(i.doi)));
      let inserted=0,skipped=0;
      for(const record of records){
        const item=makeMetadataOnlyRecord(record);
        if(item.doi&&known.has(item.doi)){skipped++;continue;}
        const r=await importLibraryItem(item);if(r==='inserted'){inserted++;if(item.doi)known.add(item.doi);}else skipped++;
      }
      await reload();onNotify(`引用导入：新增 ${inserted}，跳过 ${skipped}。PDF 附件未包含在 RIS/BibTeX 中。`);
    }catch(error){onNotify(`引用导入失败：${stringifyError(error)}`,'error');}
  },[snapshot,reload,onNotify]);

  return <div className="literature-workspace">
    <input ref={citationInput} type="file" className="hidden" accept=".bib,.ris" onChange={e=>{void importCitations(e.target.files?.[0]||null);e.target.value='';}} />
    <aside className="literature-folders">
      <div className="literature-panel-title"><Database size={17}/> 文献分类 <button className="icon-button" title="创建分类" onClick={()=>void createFolder()}><FolderPlus size={17}/></button></div>
      <button className={`lib-category ${scope==='all'?'active':''}`} onClick={()=>setScope('all')}><FileText size={16}/> 全部文献 <em>{snapshot.items.length}</em></button>
      <button className={`lib-category ${scope==='starred'?'active':''}`} onClick={()=>setScope('starred')}><Star size={16}/> 星标文献</button>
      <div className="lib-divider">分类文件夹 {scope.startsWith('collection:')&&<button title="删除选中分类" onClick={()=>void removeFolder()}><Trash2 size={13}/></button>}</div>
      {folderTree.length===0?<p className="lib-small-note">点击右侧 + 创建分类</p>:folderTree.map(({item,depth})=><button key={item.id}
        className={`lib-category ${scope==='collection:'+item.id?'active':''}`} style={{paddingLeft:14+depth*13}}
        onClick={()=>setScope('collection:'+item.id)}><Folder size={15}/><span>{item.name}</span></button>)}
      <div className="lib-divider">标签</div>
      {allTags.length===0?<p className="lib-small-note">编辑论文详情可添加标签</p>:allTags.map(tag=><button key={tag} className={`lib-category ${scope==='tag:'+tag?'active':''}`} onClick={()=>setScope('tag:'+tag)}><Tag size={14}/>{tag}</button>)}
      <div className="lib-folder-footer">PDF 文件保留原位置 · 本机 SQLite</div>
    </aside>
    <section className="literature-list-col">
      <div className="literature-title-row"><div><h2>科研文献库</h2><span>{snapshot.items.length} 篇文献 · 当前显示 {filtered.length} 篇</span></div>
        <div className="lib-top-actions"><button className="button primary" disabled={!!progress} onClick={()=>void importFiles(false)}><FilePlus2 size={15}/> 导入 PDF</button>
        <button className="button" disabled={!!progress} onClick={()=>void importFiles(true)}><FolderPlus size={15}/> 导入文件夹</button></div></div>
      <div className="literature-query-row"><Search size={17}/><input placeholder="搜索标题、作者、DOI、标签、摘要…" value={query} onChange={e=>setQuery(e.target.value)}/>
        <label title="搜索已建立的 PDF 全文索引"><input type="checkbox" checked={searchFullText} onChange={e=>setSearchFullText(e.target.checked)}/> 全文</label><ListFilter size={16}/></div>
      <div className="literature-bulk-actions"><button onClick={()=>isTauri()?void importCitations(null):citationInput.current?.click()}><ArrowDownToLine size={14}/> 导入 BibTeX/RIS</button>
        <button onClick={()=>void exportItems('bib')}>导出 BibTeX</button><button onClick={()=>void exportItems('ris')}>导出 RIS</button><button onClick={()=>void exportItems('json')}>元数据备份 JSON</button>
        <button title="刷新文献库" onClick={()=>void reload()}><RefreshCw size={14}/></button></div>
      {progress&&<div className="lib-progress"><div className="lib-progress-heading"><LoaderCircle className="spin" size={16}/>{progress.phase}: {progress.done}/{progress.total}
        <button onClick={()=>{cancelRef.current=true}}>取消后续任务</button></div><div className="lib-progress-track"><div style={{width:`${100*progress.done/Math.max(1,progress.total)}%`}}/></div>
        <span>新增 {progress.imported} · 跳过 {progress.skipped} · 失败 {progress.failed}</span></div>}
      {errors.length>0&&<details className="lib-error-list"><summary>{errors.length} 项错误（可展开查看）</summary>{errors.map((e,i)=><p key={i}>{e}</p>)}</details>}
      <div className="lib-table-head"><span>文献标题 / 作者</span><span>年份</span><span>状态</span></div>
      <div className="lib-rows">{loading?<div className="lib-empty">正在读取 SQLite 文献数据库…</div>:
        filtered.length===0?<div className="lib-empty"><FileSearch size={34}/><strong>{query?'没有找到匹配文献':'文献库还是空的'}</strong><p>导入 PDF 或 BibTeX/RIS，开始整理科研资料。</p></div>:
        filtered.map(item=><button key={item.id} className={`lib-paper-row ${selectedId===item.id?'selected':''}`} onClick={()=>setSelectedId(item.id)} onDoubleClick={()=>item.path?void onOpenPdf(item.path):onNotify('这是一条引用元数据，没有关联 PDF 附件','error')}>
          <span className="lib-paper-main"><span className="lib-paper-title">{item.starred&&<Star size={13} fill="#e8ae3a" color="#e8ae3a"/>}{item.title||item.filename}</span><small>{item.authors||item.filename||'作者待补充'}</small><small>{item.journal||item.doi||'尚无期刊信息'}</small></span>
          <span className="lib-paper-year">{item.year||'—'}</span><span className="lib-paper-status">{item.path?<FileText size={17}/>:<BookOpen size={17}/>}</span></button>)}
      </div>
    </section>
    <aside className="literature-detail">
      {!selected||!draft?<div className="lib-detail-empty"><BookOpen size={36}/><h3>文献详细信息</h3><p>点击中间列表中的论文，查看和编辑标题、作者、DOI、标签与分类。</p></div>:<>
        <div className="lib-detail-header"><strong>文献详情</strong><button className="icon-button" title="收藏 / 取消收藏" onClick={()=>void updateSelected({starred:!selected.starred})}><Star size={17} color={selected.starred?'#e1aa32':undefined} fill={selected.starred?'#e1aa32':'none'}/></button></div>
        <div className="lib-detail-scroll"><div className="lib-selected-title">{selected.title}</div>
          <div className="lib-tagline">{selected.path?fileSize(selected.fileSize):'仅元数据'} · {selected.indexStatus==='complete'?'全文已索引':selected.indexStatus==='partial'?'部分索引':'未索引'}</div>
          <div className="lib-detail-actions"><button className="button primary" disabled={!selected.path} onClick={()=>selected.path&&void onOpenPdf(selected.path)}><BookOpen size={15}/> 阅读 PDF</button>
            <button className="button" onClick={()=>setEditing(e=>!e)}><Pencil size={15}/>{editing?'取消编辑':'编辑信息'}</button></div>
          <label className="lib-field">标题<input disabled={!editing} value={draft.title} onChange={e=>setDraft({...draft,title:e.target.value})}/></label>
          <label className="lib-field">作者（用分号分隔）<input disabled={!editing} value={draft.authors} onChange={e=>setDraft({...draft,authors:e.target.value})}/></label>
          <div className="lib-field-grid"><label className="lib-field">年份<input type="number" disabled={!editing} value={draft.year??''} onChange={e=>setDraft({...draft,year:e.target.value?Number(e.target.value):null})}/></label>
            <label className="lib-field">DOI<input disabled={!editing} value={draft.doi} onChange={e=>setDraft({...draft,doi:e.target.value})}/></label></div>
          <label className="lib-field">期刊 / 会议<input disabled={!editing} value={draft.journal} onChange={e=>setDraft({...draft,journal:e.target.value})}/></label>
          <label className="lib-field">摘要<textarea rows={4} disabled={!editing} value={draft.abstract} onChange={e=>setDraft({...draft,abstract:e.target.value})}/></label>
          <label className="lib-field">标签（用英文逗号分隔）<input disabled={!editing} value={draft.tags.join(', ')} onChange={e=>setDraft({...draft,tags:e.target.value.split(',').map(x=>x.trim())})}/></label>
          <div className="lib-collection-selector"><div>所属分类</div>{snapshot.collections.map(c=><label key={c.id}><input type="checkbox" disabled={!editing} checked={draft.collectionIds.includes(c.id)} onChange={e=>setDraft({...draft,collectionIds:e.target.checked?[...draft.collectionIds,c.id]:draft.collectionIds.filter(id=>id!==c.id)})}/>{c.name}</label>)}</div>
          {editing&&<button className="button primary" onClick={()=>void saveDraft()}><Check size={15}/> 保存论文信息</button>}
          <div className="lib-detail-actions"><button className="button" disabled={!selected.path||!!progress} onClick={()=>void inspectMetadata()}><FileSearch size={15}/> 识别 PDF</button>
            <button className="button" disabled={!draft.doi} onClick={()=>void onlineLookup()}><RefreshCw size={15}/> Crossref</button></div>
          <div className="lib-detail-actions"><button className="button" disabled={!selected.path||!!progress} onClick={()=>void indexSelected()}><Search size={15}/> {selected.indexStatus==='complete'?'重建全文索引':'建立全文索引'}</button></div>
          <p className="lib-detail-hint">DOI 补全需要联网并经过确认；全文索引在本机运行，不发送论文内容。</p>
          <button className="lib-delete" onClick={()=>void deleteSelected()}><Trash2 size={14}/> 从文献库移除（保留文件和批注）</button>
        </div>
      </>}
    </aside>
  </div>;
}
