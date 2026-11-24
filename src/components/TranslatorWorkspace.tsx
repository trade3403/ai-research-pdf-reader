import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { CheckCircle2, ClipboardCopy, KeyRound, Languages, Save, Settings2, Trash2, X } from 'lucide-react';
import ResearchMarkdown from './ResearchMarkdown';
import { findSavedTranslation, getMonthlyUsage, hasAzureTranslatorKey, loadTranslatorSettings, monthId, removeAzureTranslatorKey, saveTranslatorSettings, setAzureTranslatorKey, translateResearchContent } from '../utils/translatorStore';
import type { SavedTranslation, TranslatorSettings } from '../utils/translatorStore';

type Props = { source: string; label: string; onClose: () => void; onNotify: (message: string, tone?: 'info' | 'error') => void; };

export default function TranslatorWorkspace({ source, label, onClose, onNotify }: Props) {
  const [settings, setSettings] = useState<TranslatorSettings | null>(null);
  const [keyInput, setKeyInput] = useState('');
  const [keyReady, setKeyReady] = useState(false);
  const [glossaryText, setGlossaryText] = useState('');
  const [result, setResult] = useState<SavedTranslation | null>(null);
  const [cacheHit, setCacheHit] = useState(false);
  const [used, setUsed] = useState(0);
  const [busy, setBusy] = useState(false);
  const [showSettings, setShowSettings] = useState(!source.trim());
  const [ready, setReady] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    let live = true;
    (async () => {
      try {
        const [config, hasKey, chars] = await Promise.all([loadTranslatorSettings(), hasAzureTranslatorKey(), getMonthlyUsage()]);
        if (!live) return;
        setSettings(config); setKeyReady(hasKey); setUsed(chars);
        setGlossaryText(config.glossary.map(x => `${x.source} = ${x.target}`).join('\n'));
        setReady(true);
      } catch (e) {if (live) {setError(String(e));setReady(true);} }
    })();
    return () => { live = false; };
  }, []);

  useEffect(() => {
    if (!settings || !source.trim()) {setResult(null);return;}
    let live=true;
    void findSavedTranslation(source,settings).then(item=>{if(live){setResult(item);setCacheHit(Boolean(item));}})
      .catch(e=>{if(live)setError(String(e));});
    return()=>{live=false};
  }, [source,settings]);

  const saveOptions = async () => {
    if (!settings) return;
    setBusy(true);setError('');
    try {
      const terms=glossaryText.split(/\r?\n/).map(line=>{
        const equal=line.indexOf('=');
        if(equal<0 && line.trim()) throw new Error('词典每行使用「English = 中文」格式');
        return {source:line.slice(0,equal).trim(),target:line.slice(equal+1).trim()};
      }).filter(g=>g.source && g.target);
      const saved=await saveTranslatorSettings({...settings,glossary:terms});
      setSettings(saved);
      if(keyInput.trim()){await setAzureTranslatorKey(keyInput.trim());setKeyInput('');setKeyReady(true);}
      setShowSettings(false);
      onNotify('Azure 翻译设置和专业词典已保存');
    } catch(e){setError(String(e));}
    finally{setBusy(false);}
  };

  const removeKey=async()=>{
    if(!window.confirm('确认删除本机 Windows 加密密钥存储中的 Azure Translator Key？'))return;
    setBusy(true);try{await removeAzureTranslatorKey();setKeyReady(false);setKeyInput('');onNotify('Azure Key 已从 Windows 加密密钥存储删除');}
    catch(e){setError(String(e))}finally{setBusy(false)}
  };

  const translate = async () => {
    if(!settings)return;
    setBusy(true);setError('');
    try{
      const translated=await translateResearchContent(source,settings);
      setResult(translated.result);setCacheHit(translated.cached);setUsed(translated.used);
      onNotify(translated.cached?'已读取本地翻译缓存':'译文已保存到本机 SQLite');
    }catch(e){setError(e instanceof Error?e.message:String(e));setUsed(await getMonthlyUsage().catch(()=>used));}
    finally{setBusy(false)}
  };

  const copy = async () => {
    if(!result)return;
    try {
      if(isTauri()){const {writeText}=await import('@tauri-apps/plugin-clipboard-manager');await writeText(result.translated);}
      else await navigator.clipboard.writeText(result.translated);
      onNotify('译文已复制');
    }catch(e){onNotify(`复制失败：${String(e)}`,'error')}
  };

  return <div className="translator-backdrop" role="dialog" aria-modal="true" aria-label="Azure 科研翻译">
    <div className="translator-window">
      <div className="translator-header"><div><Languages size={19}/><strong>科研翻译 · Azure Translator</strong><small>{label}</small></div>
        <div><button type="button" className="button" onClick={()=>setShowSettings(x=>!x)}><Settings2 size={14}/> 翻译设置</button>
          <button type="button" className="icon-button" title="关闭翻译窗口" onClick={onClose}><X size={18}/></button></div>
      </div>
      {!ready && <p className="translator-note">正在读取翻译配置…</p>}
      {settings && <>
        {showSettings && <section className="translator-config">
          <div className="translator-config-grid">
            <label>资源区域（全球资源可选 global）
              <input value={settings.region} onChange={e=>setSettings({...settings,region:e.target.value})} placeholder="global / eastasia / westus"/></label>
            <label>每月本机预算（字符）
              <input type="number" min="1000" max="2000000" value={settings.monthlyLimit} onChange={e=>setSettings({...settings,monthlyLimit:Number(e.target.value)})}/></label>
            <label>源语言
              <select value={settings.from} onChange={e=>setSettings({...settings,from:e.target.value as TranslatorSettings['from']})}><option value="auto">自动识别</option><option value="en">英语</option><option value="zh-Hans">简体中文</option></select></label>
            <label>目标语言
              <select value={settings.to} onChange={e=>setSettings({...settings,to:e.target.value as TranslatorSettings['to']})}><option value="zh-Hans">简体中文</option><option value="en">英语</option></select></label>
          </div>
          <label className="translator-key-field"><span><KeyRound size={14}/> Azure API Key（仅保存到 Windows 加密密钥存储）</span>
            <input type="password" autoComplete="off" placeholder={keyReady?'已配置；留空则保留原有 Key':'粘贴 Azure Translator Key'} value={keyInput} onChange={e=>setKeyInput(e.target.value)}/></label>
          <label>科研术语词典（每行：英文术语 = 中文译法）
            <textarea rows={4} value={glossaryText} onChange={e=>setGlossaryText(e.target.value)} placeholder={'stick-slip = 粘滑\na posteriori = 后验'}/></label>
          <div className="translator-config-actions"><span>{keyReady?'已配置 Azure Key':'尚未配置 Azure Key'}</span>
            {keyReady && <button type="button" className="button" disabled={busy} onClick={()=>void removeKey()}><Trash2 size={13}/> 删除 Key</button>}
            <button type="button" className="button primary" disabled={busy} onClick={()=>void saveOptions()}><Save size={14}/> 保存设置</button></div>
          <p className="translator-note">仅在点击「开始翻译」时，将需要翻译的自然语言发送给 Azure。公式、代码和术语固定译法由本机处理；词典替换可能影响少量句子流畅度。</p>
        </section>}
        <div className="translator-summary">
          <span>UTC {monthId()} 本机已预留：<strong>{used.toLocaleString()}</strong> / {settings.monthlyLimit.toLocaleString()} 字符</span>
          <span>实际 Azure 使用量请在 Azure Portal 核实</span>
        </div>
        {source.trim() && <div className="translator-content">
          <section className="translator-column"><div className="translator-column-title">原文</div><div className="translator-rich"><ResearchMarkdown source={source}/></div></section>
          <section className="translator-column"><div className="translator-column-title">译文 {result&&<span><CheckCircle2 size={14}/> {cacheHit?'本地缓存':'已保存 SQLite'}</span>}</div>
            <div className="translator-rich">{result?<ResearchMarkdown source={result.translated}/>:<p className="translator-empty">选择「开始翻译」后，这里显示排版后的译文。</p>}</div></section>
        </div>}
        {error && <p className="translator-error" role="alert">{error}</p>}
        <div className="translator-footer">
          <span>仅主动翻译 · 原文不修改 · 单次最多 2.5 万字符</span>
          {result&&<button className="button" onClick={()=>void copy()}><ClipboardCopy size={14}/> 复制译文</button>}
          {source.trim()&&<button className="button primary" disabled={busy||!keyReady&&!result} onClick={()=>void translate()}><Languages size={15}/>{busy?'正在翻译…':result?'重新查看／翻译':'开始翻译'}</button>}
        </div>
      </>}
    </div>
  </div>;
}
