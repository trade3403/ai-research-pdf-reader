import DOMPurify from 'dompurify';

export type ImportedWord = {html:string; warnings:string[]};
const MAX_IMPORT_BYTES = 16*1024*1024;
/** DOCX is a ZIP container. External URLs and active HTML never enter the editor. */
export async function importDocxBytes(bytes: Uint8Array): Promise<ImportedWord> {
  if (bytes.length<4 || bytes.length>MAX_IMPORT_BYTES) throw new Error('只支持小于 16 MB 的 DOCX 文件');
  if (bytes[0]!==0x50 || bytes[1]!==0x4b) throw new Error('文件不是有效的 DOCX/ZIP');
  const mammoth = await import('mammoth');
  const arrayBuffer = bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength) as ArrayBuffer;
  const result = await mammoth.convertToHtml({arrayBuffer}, {
    convertImage: mammoth.images.imgElement(async image => ({src:await image.read('base64').then(base64=>`data:${image.contentType};base64,${base64}`)})),
  });
  const clean=DOMPurify.sanitize(result.value, {USE_PROFILES:{html:true},FORBID_TAGS:['iframe','object','embed','svg','math','form','script','style'],FORBID_ATTR:['srcset','style'],ALLOW_DATA_ATTR:false});
  const element=document.createElement('div');element.innerHTML=clean;
  element.querySelectorAll('a').forEach(a=>{
    if (!/^https:\/\//i.test(a.href)) a.removeAttribute('href');
    a.removeAttribute('target');a.removeAttribute('rel');
  });
  element.querySelectorAll('img').forEach(image=>{
    const src=image.getAttribute('src')||'';
    if(!/^data:image\/(png|jpeg|gif|webp);base64,[a-z0-9+/=]+$/i.test(src) || src.length>3_000_000) image.remove();
  });
  return {html:element.innerHTML,warnings:result.messages.map(m=>m.message)};
}
