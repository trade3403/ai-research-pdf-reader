const { _electron: electron } = require('playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
(async () => {
  const out = path.resolve(__dirname,'../gui-evidence');
  await fs.mkdir(out,{recursive:true});
  const app = await electron.launch({args:[path.resolve(__dirname,'..')]});
  let timer, pending = Promise.resolve(), count = 0, captureError, page;
  const diagnostics = [];
  try {
    page = await app.firstWindow();
    page.on('console', msg => { if (['error','warning'].includes(msg.type())) diagnostics.push(msg.type()+': '+msg.text()); });
    page.on('pageerror', error => diagnostics.push('pageerror: '+error.message));
    page.on('requestfailed', request => diagnostics.push('requestfailed: '+request.url()+' '+request.failure()?.errorText));
    const pdfInput = page.locator('input[type=file][accept="application/pdf,.pdf"]');
    await pdfInput.waitFor({state:'attached',timeout:30000});
    await page.locator('.status-bar').getByText('尚未打开文档',{exact:true}).waitFor();
    await page.screenshot({path:path.join(out,'01-start.png')});
    // Record actual Electron renderer screenshots at a fixed cadence; no mock UI.
    const capture = () => {
      pending = pending.then(async () => {
        await page.screenshot({path:path.join(out,'frame-'+String(count++).padStart(5,'0')+'.png'),timeout:5000});
      }).catch(e => {captureError = e;});
    };
    capture(); timer=setInterval(capture,500);
    await page.waitForTimeout(1500);
    const objects = ['<< /Type /Catalog /Pages 2 0 R >>','<< /Type /Pages /Kids [3 0 R] /Count 1 >>','<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>','<< /Length 55 >>\nstream\nBT /F1 20 Tf 72 720 Td (RPCore test paper) Tj ET\nendstream','<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>'];
    const stream='BT /F1 20 Tf 72 720 Td (RPCore test paper) Tj ET\n';
    objects[3]='<< /Length '+Buffer.byteLength(stream)+' >>\nstream\n'+stream+'endstream';
    let pdf='%PDF-1.4\n'; const offsets=[];
    objects.forEach((obj,i)=>{offsets.push(Buffer.byteLength(pdf));pdf+=(i+1)+' 0 obj\n'+obj+'\nendobj\n';});
    const xref=Buffer.byteLength(pdf);
    pdf+='xref\n0 6\n0000000000 65535 f \n'+offsets.map(x=>String(x).padStart(10,'0')+' 00000 n \n').join('')+'trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n'+xref+'\n%%EOF\n';
    await page.getByRole('button',{name:'PDF 阅读',exact:true}).click();
    await pdfInput.setInputFiles({name:'GUI-fixture.pdf',mimeType:'application/pdf',buffer:Buffer.from(pdf)});
    await page.locator('.status-bar').getByText('1 页 · GUI-fixture.pdf',{exact:true}).waitFor({timeout:30000});
    await page.getByRole('button',{name:'PDF 阅读',exact:true}).click();
    await page.locator('canvas').first().waitFor({state:'visible',timeout:30000});
    await page.getByText('RPCore test paper',{exact:true}).first().waitFor({timeout:30000});
    await page.waitForTimeout(2000);
    await page.screenshot({path:path.join(out,'02-pdf-open.png')});
    assert.ok(await page.locator('canvas').count());
    await page.waitForTimeout(2000);
  } catch (error) {
    diagnostics.push(String(error.stack || error));
    clearInterval(timer); await pending;
    if (page) {
      await page.screenshot({path:path.join(out,'failure.png'),timeout:5000}).catch(()=>{});
      const body = await page.locator('body').innerText({timeout:5000}).catch(()=> 'unavailable');
      diagnostics.push('Visible page: '+body);
    }
    throw error;
  } finally {
    clearInterval(timer); await pending;
    await fs.writeFile(path.join(out,'diagnostics.txt'), diagnostics.join('\n'));
    await app.close();
  }
  if(captureError) throw captureError;
  assert.ok(count>=4,'insufficient captured frames');
})().catch(e=>{console.error(e);process.exitCode=1;});
