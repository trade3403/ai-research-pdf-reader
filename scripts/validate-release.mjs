import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const version=JSON.parse(readFileSync(new URL('../package.json',import.meta.url),'utf8')).version;
const tag=process.env.GITHUB_REF_TYPE==='tag'?process.env.GITHUB_REF_NAME:'';
if(tag){
  assert.match(tag,/^v\d+\.\d+\.\d+$/,'Only stable semantic versions may be published');
  assert.equal(tag,`v${version}`,'Release tag must match package.json and Tauri versions');
  const tauri=JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json',import.meta.url),'utf8'));
  assert.equal(tauri.version,version,'Tauri installer version must match package.json');
}
console.log(tag?`Release tag validated: ${tag}`:`CI branch build v${version}; no release tag requested`);
