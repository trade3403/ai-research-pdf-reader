const fs = require('node:fs'); const path = require('node:path');
const src = path.resolve(__dirname, '../../dist');
const dest = path.resolve(__dirname, '../workspace');
if (!fs.existsSync(path.join(src,'index.html'))) throw new Error('Build React at repository root first');
fs.rmSync(dest,{recursive:true,force:true}); fs.cpSync(src,dest,{recursive:true});
