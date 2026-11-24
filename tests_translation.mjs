import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { assembleResearchTranslation, splitResearchMarkup, translationPayload } from './src/utils/translationMarkup.ts';
const source = String.raw`## Stability discussion
The discrete energy satisfies \(E^{n+1}-E^n \le 0\), with stick-slip contact.

$$
F = kx
$$

\`\`\`python
print("the interface remains stable")
\`\`\`

The next paragraph remains translatable.`.replaceAll('\\`','`');
const glossary=[{source:'stick-slip',target:'粘滑'}];
const segments=splitResearchMarkup(source,glossary);
const payload=translationPayload(segments);
assert.ok(!payload.some(t=>t.includes('E^{n+1}')||t.includes('F = kx')||t.includes('print(')||t.includes('stick-slip')),'LaTeX/code/glossary must not be sent');
assert.ok(payload.some(t=>t.includes('The discrete energy satisfies')));
const assembled=assembleResearchTranslation(segments,payload.map(s=>`译:${s}`));
assert.ok(assembled.includes(String.raw`\(E^{n+1}-E^n \le 0\)`));
assert.ok(assembled.includes('$$\nF = kx\n$$'));
assert.ok(assembled.includes('```python\nprint('));
assert.ok(assembled.includes('粘滑'));
const inline=splitResearchMarkup('Test `$foo` and $x^2$ and `let x=42`');
assert.deepEqual(translationPayload(inline).map(x=>x.trim()).filter(Boolean),['Test','and','and']);
assert.throws(()=>assembleResearchTranslation(segments,[]));
const read=p=>readFileSync(new URL(p,import.meta.url),'utf8');
const app=read('./src/App.tsx'),viewer=read('./src/components/TranslatorWorkspace.tsx');
const store=read('./src/utils/translatorStore.ts'),rust=read('./src-tauri/src/lib.rs');
const cargo=read('./src-tauri/Cargo.toml'),cap=JSON.parse(read('./src-tauri/capabilities/default.json'));
assert.match(app,/selection-translate/);
assert.match(app,/setTranslationDraft/);
assert.match(read('./src/components/ResearchRecords.tsx'),/翻译 \/ 中英对照/);
assert.match(store,/CREATE TABLE IF NOT EXISTS translation_cache/);
assert.match(store,/CREATE TABLE IF NOT EXISTS translation_usage/);
assert.match(store,/used_chars\+\$1<=\$3/);
assert.match(store,/crypto\.subtle\.digest\('SHA-256'/);
assert.match(viewer,/ResearchMarkdown/);
assert.match(viewer,/Windows 凭据管理器/);
assert.match(rust,/const TRANSLATOR_SERVICE/);
assert.match(rust,/reqwest::Client::builder/);
assert.match(rust,/translator_translate/);
assert.doesNotMatch(store,/localStorage\.setItem\([^\n]*keyInput/);
assert.match(cargo,/windows-native/);
assert.deepEqual(cap.webviews,['main']);
console.log('Azure translator markup preservation, privacy and quota tests passed');
