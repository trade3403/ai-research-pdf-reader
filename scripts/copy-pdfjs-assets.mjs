import { cpSync, existsSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const sourceRoot = join(projectRoot, 'node_modules', 'pdfjs-dist');
const publicRoot = join(projectRoot, 'public');

for (const folder of ['cmaps', 'standard_fonts', 'wasm']) {
  const source = join(sourceRoot, folder);
  const target = join(publicRoot, folder);
  if (existsSync(source)) {
    mkdirSync(publicRoot, { recursive: true });
    cpSync(source, target, { recursive: true, force: true });
    console.log(`PDF.js asset copied: ${folder}`);
  } else {
    console.warn(`PDF.js asset directory not found (optional): ${source}`);
  }
}
