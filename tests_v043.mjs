import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {buildResearchPrompt} from './src/utils/researchPrompts.ts';
const read=p=>readFileSync(new URL(p,import.meta.url),'utf8');
for(const kind of ['explain','math','novelty','python']){
 const prompt=buildResearchPrompt(kind,'E = 0','paper.pdf',3);
 assert.match(prompt,/paper.pdf/);assert.match(prompt,/第?3|页码：3/);assert.match(prompt,/E = 0/);
}
assert.match(buildResearchPrompt('novelty','text','a',1),/不能证明原创性/);
assert.throws(()=>buildResearchPrompt('math','   ','a',1));
const app=read('./src/App.tsx'),pdf=read('./src/components/PdfPage.tsx'),dock=read('./src/components/ChatBrowserDock.tsx'),rust=read('./src-tauri/src/lib.rs');
assert.match(pdf,/onContextMenu/);assert.match(pdf,/event.preventDefault\(\)/);
assert.match(app,/pdf-research-context-menu/);assert.match(app,/promptForSelection/);
assert.match(app,/saveSelectionAsResearch/);assert.match(app,/VACUUM INTO \$1/);
assert.match(dock,/preparedPrompt/);assert.match(dock,/writeText\(preparedPrompt\)/);
assert.doesNotMatch(dock,/executeJavaScript|document\.cookie|querySelector\(.+chatgpt/);
assert.match(rust,/fn stage_database_restore/);assert.match(rust,/fn restore_staged_database/);
assert.match(rust,/app\.path\(\)\.app_config_dir/);
assert.match(rust,/research-reader-before-restore.sqlite/);
assert.match(rust,/backup_prepare_target/);
const cap=JSON.parse(read('./src-tauri/capabilities/default.json'));
assert.deepEqual(cap.webviews,['main']);
assert.match(read('./src-tauri/tauri.conf.json'),/0\.(?:4\.3|5\.[012356789])/);
console.log('V0.4.3 context actions, clipboard, provenance and backup/restore checks passed');
