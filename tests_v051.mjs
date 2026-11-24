import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const read=p=>readFileSync(new URL(p,import.meta.url),'utf8');
const {academicIndex,crossReferenceText,citationText,referenceText,ensureAcademicIds}=await import('./src/utils/wordAcademic.ts');
let sequence=0;const next=()=>`academic-${++sequence}`;
const sample={type:'doc',content:[
  {type:'wordBlockMath',attrs:{latex:'a=b'}},
  {type:'wordCaption',attrs:{kind:'figure',text:'A test figure'}},
  {type:'paragraph',content:[{type:'wordCitation',attrs:{paperId:'p1',authors:'Jane Smith',title:'Stability',year:2024,journal:'J Math',doi:'10.1000/a'}},
    {type:'wordCrossReference',attrs:{targetId:'academic-2'}},
    {type:'wordCitation',attrs:{paperId:'p1',authors:'Jane Smith',title:'Stability',year:2024,journal:'J Math',doi:'10.1000/a'}}]},
  {type:'wordBlockMath',attrs:{latex:'x=y'}},
  {type:'wordCaption',attrs:{kind:'table',text:'Results'}},
  {type:'wordBibliography'},
]};
const document=ensureAcademicIds(sample,next);assert.equal(document.content[0].attrs.id,'academic-1');
const i1=academicIndex(document);
assert.deepEqual(i1.targets.map(t=>t.label),['Equation 1','Figure 1','Equation 2','Table 1']);
assert.equal(i1.citations.length,1);
assert.equal(crossReferenceText('academic-2',i1),'Figure 1');
assert.equal(citationText(document.content[2].content[0],i1,'numeric'),'[1]');
assert.equal(citationText(document.content[2].content[0],i1,'author-year'),'(Smith, 2024)');
assert.match(referenceText(i1.citations[0],'numeric',1),/\[1\].*Stability.*doi\.org/);
const i2=academicIndex({type:'doc',content:[...document.content.slice(0,1),...document.content.slice(4)]});
assert.deepEqual(i2.targets.map(t=>t.label),['Equation 1','Table 1']);
assert.equal(crossReferenceText('academic-2',i2),'[Missing reference]');
const unchanged=ensureAcademicIds(document,next);assert.equal(sequence,4,'IDs not reallocated on every render');
assert.deepEqual(unchanged,document);
const duplicate={type:'doc',content:[{type:'wordBlockMath',attrs:{id:'same'}},{type:'wordBlockMath',attrs:{id:'same'}}]};
const deduped=ensureAcademicIds(duplicate,next);assert.notEqual(deduped.content[0].attrs.id,deduped.content[1].attrs.id);
const editor=read('./src/components/WordEditor.tsx');for(const item of ['WordCaption','WordCitation','WordCrossReference','WordBibliography','insertCrossRef','insertCaption','lineSpacing','headerText'])assert.ok(editor.includes(item),item);
const store=read('./src/utils/wordStore.ts');assert.match(store,/PRAGMA table_info\(word_manuscripts\)/);
assert.match(store,/ALTER TABLE word_manuscripts ADD COLUMN/);
const exportSrc=read('./src/utils/wordExport.ts');for(const item of ['PageNumber.CURRENT','new Header','new Footer','academicIndex','crossReferenceText','referenceText'])assert.ok(exportSrc.includes(item),item);
const workflow=read('./.github/workflows/build-windows-installer.yml');assert.match(workflow,/test:v051/);
assert.match(workflow,/Validate stable release tag/);
assert.match(read('./.github/workflows/restore-code-version.yml'),/gh pr create/);
assert.match(read('./scripts/validate-release.mjs'),/GITHUB_REF_TYPE/);
assert.match(read('./.github/workflows/publish-verified-release.yml'),/sha256sum -c SHA256SUMS\.txt/);
const pkg=JSON.parse(read('./package.json'));assert.ok(['0.5.1','0.5.2','0.5.3','0.5.5','0.5.6','0.5.7','0.5.8','0.5.9'].includes(pkg.version));
console.log('V0.5.1 stable academic IDs, auto numbering, citations, layout migration and release workflow checks passed');
