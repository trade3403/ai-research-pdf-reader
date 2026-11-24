import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { normalizeResearchMath } from './src/utils/normalizeResearchMath.ts';
const sample = String.raw`Error is \(u-u_h\) and it satisfies \[\boxed{E\le\eta}\].`;
assert.equal(normalizeResearchMath(sample), String.raw`Error is $u-u_h$ and it satisfies $$\boxed{E\le\eta}$$.`);
assert.equal(normalizeResearchMath(String.raw`Inline $x^2$ and $$y^2$$`), String.raw`Inline $x^2$ and $$y^2$$`);
assert.equal(normalizeResearchMath('`\\(x\\)`'), '`\\(x\\)`');
const fenced = String.raw`~~~latex
\[x\]
~~~`;
assert.equal(normalizeResearchMath(fenced), fenced);
const read = name => readFileSync(new URL(name, import.meta.url),'utf8');
const view = read('./src/components/ResearchMarkdown.tsx');
assert.match(view, /remarkMath/);
assert.match(view, /rehypeKatex/);
assert.match(view, /skipHtml/);
assert.match(view, /trust: false/);
const panel = read('./src/components/ResearchRecords.tsx');
assert.match(panel, /排版预览/);
assert.match(panel, /编辑原文/);
assert.match(panel, /分栏/);
console.log('Research Markdown/LaTeX rendering and security tests passed');
