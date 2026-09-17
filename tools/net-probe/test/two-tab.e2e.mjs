import { chromium } from 'playwright';
// Usage: node test/dev-relay.mjs const browser npx http-server dist -p 8765 const browser node test/two-tab.e2e.mjs
const browser = await chromium.launch({
  executablePath: '/opt/pw-browsers/chromium',
  args: ['--disable-features=WebRtcHideLocalIpsWithMdns', '--ignore-certificate-errors'],
});
const mk = async (name) => {
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: 390, height: 844 } });
  const p = await ctx.newPage();
  p.on('console', (m) => console.log(name, m.type(), m.text().slice(0, 200)));
  p.on('pageerror', (e) => console.log(name, 'PAGEERROR', e.message));
  return p;
};
const a = await mk('A');
await a.goto('http://127.0.0.1:8765/?relay=ws://127.0.0.1:7777');
await a.waitForFunction(() => !document.getElementById('natVerdict').textContent.includes('검사 중'), null, { timeout: 15000 });
console.log('A nat:', await a.textContent('#natVerdict'), '|', await a.textContent('#natDetail'));
await a.click('#create');
const link = await a.inputValue('#linkShow');
console.log('link', link);
const b = await mk('B');
await b.goto(link);
for (let i = 0; i < 20; i++) {
  await new Promise((r) => setTimeout(r, 2000));
  const relays = await a.textContent('#relays');
  const pa = await a.textContent('#peers');
  if (i % 5 === 0) console.log(i * 2, 's', relays, '|', (await a.textContent('#waitHint')), '|', pa.replace(/\s+/g, ' ').slice(0, 300));
  if (pa.includes('측정 완료')) break;
}
console.log('A peers:', (await a.textContent('#peers')).replace(/\s+/g, ' '));
console.log('B peers:', (await b.textContent('#peers')).replace(/\s+/g, ' '));
console.log('row:', await a.inputValue('#mdRow'));
await a.screenshot({ path: 'shotA.png', fullPage: true });
console.log('log A:', (await a.textContent('#log')).slice(0, 600));
await browser.close();
