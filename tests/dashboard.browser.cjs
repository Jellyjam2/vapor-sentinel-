/* Real Chromium checks. The dashboard itself has no npm runtime dependencies. */
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { pathToFileURL } = require('node:url');
const { execFileSync } = require('node:child_process');
const { chromium } = require('playwright');
const model = require('../dashboard/evidence.js');
const root = path.resolve(__dirname, '..');
const artifacts = path.join(root, 'browser-artifacts');
const executable = process.env.VAPOR_SENTINEL_TEST_BIN || path.join(root, 'target/debug', `vapor_project${process.platform === 'win32' ? '.exe' : ''}`);
const env = {...process.env, VAPOR_SENTINEL_ENABLE_ACTIONS:'0'};
for (const key of ['VAPOR_SENTINEL_CONFIG','VAPOR_SENTINEL_ONESHOT','VAPOR_SENTINEL_EXIT']) delete env[key];
const recording = execFileSync(executable, ['--replay','tests/fixtures/memory-sequence.json'], {cwd:root,env,encoding:'utf8',stdio:['ignore','pipe','pipe']});
let server, browser, address;
const assetNames = new Set(['index.html','styles.css','app.js','evidence.js','example.js','mark.svg']);
before(async () => {
  fs.mkdirSync(artifacts,{recursive:true});
  server = http.createServer((req,res) => {
    const name = new URL(req.url, 'http://localhost').pathname.split('/').pop() || 'index.html';
    if (!assetNames.has(name)) {res.writeHead(404);res.end();return;}
    const type = {'.html':'text/html','.css':'text/css','.js':'text/javascript','.svg':'image/svg+xml'}[path.extname(name)];
    res.writeHead(200, {'Content-Type':type+'; charset=utf-8','Cache-Control':'no-store'});
    res.end(fs.readFileSync(path.join(root,'dashboard',name)));
  });
  await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
  address = `http://127.0.0.1:${server.address().port}/`;
  browser = await chromium.launch({headless:true,...(process.env.VAPOR_BROWSER_EXECUTABLE ? {executablePath:process.env.VAPOR_BROWSER_EXECUTABLE} : {channel:process.env.VAPOR_BROWSER_CHANNEL || 'chrome'})});
});
after(async () => {await browser?.close(); if(server) await new Promise(resolve=>server.close(resolve));});
async function withPage(fn, viewport = {width:1440,height:1080}, init) {
  const context = await browser.newContext({viewport,timezoneId:'UTC',reducedMotion:'reduce'});
  if (init) await context.addInitScript(init);
  const page = await context.newPage(), errors = [], external = [];
  page.on('pageerror',error=>errors.push(error.message));
  page.on('console',message=>{if(message.type()==='error') errors.push(message.text());});
  page.on('dialog', async dialog=>{errors.push(`Unexpected dialog: ${dialog.message()}`);await dialog.dismiss();});
  page.on('request',request=>{if(!request.url().startsWith(address) && !request.url().startsWith('file:') && !request.url().startsWith('blob:')) external.push(request.url());});
  try {
    await page.goto(address,{waitUntil:'networkidle'});
    await fn(page);
    assert.deepEqual(errors,[], 'No browser JavaScript or CSP errors');
    assert.deepEqual(external,[], 'The workspace does not request external resources');
  } finally {await context.close();}
}
const importFile = (page, content, name='recording.jsonl') => page.locator('#evidence-file').setInputFiles({name,mimeType:'application/x-ndjson',buffer:Buffer.from(content)});
const value = (page,id) => page.locator('#'+id).textContent();
async function waitStatus(page, text) {await page.locator('#status').filter({hasText:text}).waitFor();}
async function checkAxe(page, label) {
  const source=fs.readFileSync(require.resolve('axe-core/axe.min.js'),'utf8');
  await page.evaluate(source);
  const result=await page.evaluate(async()=>await window.axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21aa']}}));
  fs.writeFileSync(path.join(artifacts,`accessibility-${label}.json`),JSON.stringify(result,null,2));
  assert.deepEqual(result.violations.map(v=>({id:v.id,impact:v.impact,nodes:v.nodes.map(n=>n.target)})),[], 'Automated WCAG 2.1 AA checks');
}
test('honest empty state, recording guide, and accessible controls', async()=>withPage(async page=>{
  assert.equal(await value(page,'latest-state'),'Awaiting data');
  assert.equal(await page.locator('#clear').isDisabled(),true);
  assert.equal(await page.locator('#export').isDisabled(),true);
  await page.getByRole('button',{name:'Open recording guide',exact:true}).click();
  assert.equal(await page.getByRole('dialog').isVisible(),true);
  await page.locator('#close-guide').press('Escape');
  assert.equal(await page.getByRole('dialog').isVisible(),false);
  await checkAxe(page,'empty');
  await page.screenshot({path:path.join(artifacts,'dashboard-empty.png'),fullPage:true});
}));
test('example, filters, pagination, inspection, and keyboard timeline work together',async()=>withPage(async page=>{
  await page.locator('#load-demo').click();
  assert.equal(await value(page,'recording-mode'),'EXAMPLE · REPLAY');
  assert.equal(await value(page,'observation-count'),'48');
  assert.equal(await value(page,'latest-state'),'Normal');
  assert.equal(await value(page,'latest-value'),'52');
  assert.equal(await value(page,'anomaly-count'),'6');
  assert.equal(await value(page,'delivery-count'),'2');
  assert.equal(await page.locator('#evidence-rows tr').count(),8);
  await page.screenshot({path:path.join(artifacts,'dashboard-desktop.png'),fullPage:true});
  await checkAxe(page,'loaded');
  await page.locator('#page-next').click(); assert.equal(await value(page,'page-label'),'2 / 6');
  await page.locator('[data-filter="Anomalous"]').click();
  assert.equal(await page.locator('#evidence-rows tr').count(),6);
  await page.getByRole('button',{name:'Inspect observation 26',exact:true}).click();
  assert.equal(await value(page,'state'),'Anomalous');assert.equal(await value(page,'inspector-value'),'10%');
  await page.locator('#next').click();assert.equal(await value(page,'inspector-value'),'8%');
  await page.locator('#timeline-index').press('Home');
  assert.equal(await value(page,'state'),'Unknown');
  assert.equal(await page.locator('[data-filter="all"]').getAttribute('aria-pressed'),'true');
  await page.locator('#search').fill('cooldown');assert.equal(await page.locator('#evidence-rows tr').count(),5);
  await page.locator('#search').fill('no matching evidence');assert.equal(await page.locator('#table-empty').isVisible(),true);
  assert.equal(await page.locator('#export').isDisabled(),true);
  assert.equal(await value(page,'state'),'No selection');
  await page.locator('#clear').click();assert.equal(await value(page,'latest-state'),'Awaiting data');
}));
test('actual Rust output imports, exports with delivery context, and reimports',async()=>withPage(async page=>{
  await importFile(page,recording);await waitStatus(page,'Loaded 5 observations');
  assert.equal(await value(page,'state'),'Normal');assert.equal(await value(page,'inspector-value'),'45%');
  await page.locator('[data-filter="Anomalous"]').click();
  const downloadPromise=page.waitForEvent('download');await page.locator('#export').click();const download=await downloadPromise;
  const destination=path.join(artifacts,'exported-evidence.jsonl');await download.saveAs(destination);
  const exported=fs.readFileSync(destination,'utf8'), decoded=model.decode(exported);
  assert.equal(decoded.evaluations.length,2);assert.equal(decoded.deliveries.size,1);
  await importFile(page,exported,'roundtrip.jsonl');await waitStatus(page,'Loaded 2 observations');
  assert.equal(await value(page,'observation-count'),'2');assert.equal(await value(page,'latest-state'),'Anomalous');
}));
test('invalid and oversized imports preserve data, and hostile strings stay literal',async()=>withPage(async page=>{
  await importFile(page,recording);await waitStatus(page,'Loaded 5 observations');
  for(const invalid of ['not JSON','{"schema_version":2}','']) {
    await importFile(page,invalid,'invalid.jsonl');await waitStatus(page,'Could not load');assert.equal(await value(page,'latest-value'),'45');
  }
  await importFile(page,' '.repeat(model.MAX_BYTES+1),'large.jsonl');await waitStatus(page,'exceeds 10 MiB');assert.equal(await value(page,'latest-value'),'45');
  const rows=recording.trim().split('\n').map(JSON.parse);
  for(const row of rows) if(row.evidence) row.evidence.reason='<img src=x onerror=alert(1)>';
  await importFile(page,rows.map(JSON.stringify).join('\n'),'literal.jsonl');await waitStatus(page,'Loaded 5 observations');
  assert.equal(await value(page,'reason'),'<img src=x onerror=alert(1)>');assert.equal(await page.locator('img[src="x"]').count(),0);
}));
test('stream selection cannot combine runs or units, and missing delivery never looks delivered',async()=>withPage(async page=>{
  const rows=recording.trim().split('\n').map(JSON.parse).filter(r=>r.record_type==='evaluation');
  const clone=structuredClone(rows.at(-1));clone.run_id='another-run';clone.event_id='another-event';clone.unit='MiB';clone.evidence.metric='SYSTEM_USED_MEMORY_MIB';clone.evidence.value=2048;rows.push(clone);
  await importFile(page,rows.map(JSON.stringify).join('\n'));await waitStatus(page,'across 2 streams');
  assert.equal(await value(page,'observation-count'),'1');assert.equal(await value(page,'latest-unit'),'MiB');
  assert.match(await value(page,'delivery-description'),/no recorded outcome/);
  assert.equal(await page.locator('#delivery-list [data-state="delivered"]').count(),0);
  await page.locator('#stream').selectOption('0');assert.equal(await value(page,'observation-count'),'5');assert.equal(await value(page,'latest-unit'),'%');
}));
test('a stale file read cannot replace a newer file or restore cleared data',async()=>withPage(async page=>{
  await importFile(page,'not JSON','slow.jsonl');
  await importFile(page,recording,'current.jsonl');await waitStatus(page,'Loaded 5 observations');
  await page.waitForTimeout(300);
  assert.equal(await value(page,'recording-name'),'current.jsonl');assert.equal(await page.locator('#status.error').count(),0);
  await importFile(page,recording,'slow.jsonl');await page.locator('#clear').click();await page.waitForTimeout(300);
  assert.equal(await value(page,'latest-state'),'Awaiting data');
},undefined,()=>{
  const original=File.prototype.text;
  File.prototype.text=function(){const promise=original.call(this);return this.name==='slow.jsonl'?new Promise(resolve=>setTimeout(()=>resolve(promise),200)):promise;};
}));
test('responsive layouts have no document overflow and stay keyboard accessible',async()=>withPage(async page=>{
  await page.locator('#load-demo').click();
  for(const width of [320,390,768,1024,1440]) {
    await page.setViewportSize({width,height:900});
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+1),true,`No page overflow at ${width}px`);
    if(width===390) {await page.screenshot({path:path.join(artifacts,'dashboard-mobile.png'),fullPage:true});await checkAxe(page,'mobile');}
  }
  await page.getByRole('link',{name:'Evidence',exact:true}).click();
  await page.getByRole('button',{name:'Inspect observation 48',exact:true}).press('Enter');
  assert.equal(await value(page,'selected-sequence'),'#0048');
}));
test('the same dashboard works directly from a local HTML file',async()=>withPage(async page=>{
  await page.goto(pathToFileURL(path.join(root,'dashboard/index.html')).href,{waitUntil:'load'});
  await page.locator('#load-demo').click();assert.equal(await value(page,'observation-count'),'48');
  await importFile(page,recording);await waitStatus(page,'Loaded 5 observations');
  assert.equal(await value(page,'state'),'Normal');
}));
