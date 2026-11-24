const test=require('node:test'),assert=require('node:assert/strict');
const {prepareBatch,translateBatch,createKeyStore}=require('../azure-translator.cjs');
test('Azure request validation, fixed endpoint, no redirect, reservation before request, response validation',async()=>{
 const req={texts:['science','研究'],from:'auto',to:'zh-Hans',region:'eastasia'};
 assert.throws(()=>prepareBatch({...req,texts:['x'.repeat(10001)]}));assert.throws(()=>prepareBatch({...req,region:'https://evil.test'}));
 let used=0,calls=0;
 const deps={keyStore:{read:async()=> 'test-key-only'},reserve:async n=>{used+=n;},fetchImpl:async(url,options)=>{calls++;assert.equal(used,9);assert.equal(url.origin,'https://api.cognitive.microsofttranslator.com');assert.equal(options.redirect,'error');assert.equal(options.headers['Ocp-Apim-Subscription-Region'],'eastasia');assert.deepEqual(JSON.parse(options.body),[{Text:'science'},{Text:'研究'}]);return new Response(JSON.stringify([{translations:[{text:'科学'}]},{translations:[{text:'研究'}]}]));}};
 assert.deepEqual(await translateBatch(req,deps),{translations:['科学','研究']});
 await assert.rejects(()=>translateBatch(req,{...deps,reserve:async()=>{throw Error('quota');}}),/quota/);assert.equal(calls,1);
 await assert.rejects(()=>translateBatch(req,{...deps,fetchImpl:async()=>new Response('secret provider body',{status:401})}),/Azure HTTP 401/);
 await assert.rejects(()=>translateBatch(req,{...deps,fetchImpl:async()=>new Response('[]')}),/count/);
 await assert.rejects(()=>createKeyStore('unused',{isEncryptionAvailable:()=>false},'win32').save('test-key-only'),/DPAPI/);
});
