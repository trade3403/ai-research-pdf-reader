import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { normalizePdfSelectionRects } from './src/utils/pdfSelectionGeometry.ts';
const read=path=>readFileSync(new URL(path,import.meta.url),'utf8');
const sheet={left:100,top:200,right:800,bottom:1200};
const input=[
  {left:120,top:220,right:180,bottom:236},
  {left:120,top:220,right:180,bottom:236},
  {left:178,top:220,right:240,bottom:236},
  {left:120,top:246,right:210,bottom:262},
  {left:790,top:1190,right:840,bottom:1230},
  {left:1,top:1,right:20,bottom:10},
  {left:NaN,top:220,right:250,bottom:240},
];
const out=normalizePdfSelectionRects(input,sheet);
assert.deepEqual(out,[
  {left:120,top:220,right:240,bottom:236},
  {left:120,top:246,right:210,bottom:262},
  {left:790,top:1190,right:800,bottom:1200},
], 'Repeated selection fragments must merge only within a line, and stay clipped to the PDF sheet');
assert.deepEqual(normalizePdfSelectionRects([
  {left:120,top:220,right:180,bottom:236},
  {left:520,top:220,right:580,bottom:236},
],sheet).length, 2, 'Two columns on the same row must remain separate');
assert.deepEqual(normalizePdfSelectionRects([],sheet),[]);
const page=read('./src/components/PdfPage.tsx');
const css=read('./src/styles.css');
const workflow=read('./.github/workflows/build-windows-installer.yml');
const pkg=JSON.parse(read('./package.json'));
assert.equal(pkg.version,'0.5.9');
assert.match(page,/normalizePdfSelectionRects\(Array\.from\(range\.getClientRects\(\)\), sheetBounds\)/);
assert.match(page,/span\.classList\.toggle\('pdf-search-hit'/);
assert.doesNotMatch(page,/span\.replaceChildren\(fragment\)/,'Search must not replace selection text nodes');
assert.doesNotMatch(page,/\[pdf, pageNumber, zoom, rotation, availableWidth, query\]/,'Typing a search must not rerender PDF.js text layer');
assert.match(page,/\[query, loading, error, renderVersion\]/);
assert.match(css,/--text-scale-factor/);
assert.match(css,/--font-height/);
assert.match(css,/--scale-x/);
assert.match(css,/--rotate/);
assert.match(css,/\.pdf-text-layer \.markedContent \{ display: contents; \}/);
assert.match(css,/\.pdf-text-layer\.is-rendering \{ pointer-events: none;/);
assert.match(css,/var\(--annotation-color\) 20%/);
assert.match(css,/mix-blend-mode:multiply/);
assert.match(workflow,/test:v059/);
console.log('PDF.js V0.5.9 text-layer font scaling, precise selection, annotation visibility and no-rerender search checks passed');
