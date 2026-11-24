const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {createResearchDb}=require('../research-db.cjs');
test('native research, Word and translation migration persists across reopen',()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'research-db-'));const file=path.join(root,'data.sqlite');let db=createResearchDb(file);
 const clip={id:'c',title:'科研记录',body:'LaTeX $x^2$',sourceUrl:'https://chatgpt.com/',itemId:'p',fingerprint:null,page:1,tags:'test',createdAt:'2026',updatedAt:'2026'};
 const word={id:'w',title:'论文',content:{type:'doc',content:[{type:'paragraph',content:[{type:'text',text:'research'}]}]},sourcePath:null,pageSize:'A4',marginMm:25,citationStyle:'numeric',lineSpacing:1.5,headerText:'',footerText:'',createdAt:'2026',updatedAt:'2026'};
 const settings={region:'global',from:'auto',to:'zh-Hans',monthlyLimit:1000,glossary:[{source:'test',target:'测试'}]};
 const cache={cacheKey:'a'.repeat(64),source:'test',translated:'测试',from:'auto',to:'zh-Hans',createdAt:'2026'};
 try{
 assert.throws(()=>db.call('clip.import',[clip,{...clip,id:'bad',sourceUrl:'file:///tmp/a'}]));assert.deepEqual(db.call('clip.list'),[]);
 assert.equal(db.call('clip.import',[clip]),true);db.call('clip.save',{...clip,title:'edited',createdAt:'new'});
 assert.equal(db.call('clip.import',[clip]),false);assert.equal(db.call('word.import',[word]),true);
 db.call('translation.import',{settings,cache:[cache],usage:{'2026-11':900}});
 db.call('translation.reserve',{month:'2026-11',chars:100,limit:1000});assert.throws(()=>db.call('translation.reserve',{month:'2026-11',chars:1,limit:1000}));
 db.close();db=createResearchDb(file);
 assert.equal(db.call('clip.list')[0].title,'edited');assert.equal(db.call('clip.list')[0].createdAt,'2026');assert.deepEqual(db.call('word.list'),[word]);assert.deepEqual(db.call('translation.settings'),settings);assert.deepEqual(db.call('translation.cache',cache.cacheKey),cache);assert.equal(db.call('translation.usage','2026-11'),1000);
 assert.throws(()=>db.call('sql','DROP TABLE records'));db.call('clip.delete','c');db.call('word.delete','w');assert.deepEqual(db.call('clip.list'),[]);assert.deepEqual(db.call('word.list'),[]);
 }finally{db.close();fs.rmSync(root,{recursive:true,force:true});}
});
