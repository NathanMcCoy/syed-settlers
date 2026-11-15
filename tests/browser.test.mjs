// The pages in real browsers: WebKit (Safari's engine, as on iPhones) and Chromium (Android, desktop).
// A person plays against bots only by clicking what the page lights up; a friend's link opens their
// own seat; nothing logs an error (including the security policy refusing something); phones don't
// scroll sideways.
import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import { chromium, webkit } from 'playwright';
import { start } from '../server/server.mjs';

const server = await start({ port: 0, host: '127.0.0.1', botDelay: 20 });
const base = `http://127.0.0.1:${server.address().port}`;
after(() => server.close());

const BROWSERS = { chromium, webkit };

async function open(type, viewport = { width: 1200, height: 900 }) {
  const browser = await type.launch();
  const page = await browser.newPage({ viewport });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  return { browser, page, errors };
}

for (const [name, type] of Object.entries(BROWSERS)) {
  test(`${name}: start a game against bots and play it by clicking`, async () => {
    const { browser, page, errors } = await open(type);
    try {
      await page.goto(base + '/');
      await page.fill('input[name="name"]', 'Tester');
      await page.click('button[type="submit"]');
      await page.waitForSelector('#board svg');
      const clicks = { corner: 0, edge: 0, hex: 0, roll: 0, trade: 0, end: 0, other: 0 };
      for (let i = 0; i < 600 && clicks.roll < 8; i++) {
        const state = await page.evaluate(() => ({ phase: document.body.dataset.phase, mine: document.body.dataset.myTurn === 'true' }));
        if (state.phase === 'over') break;
        if (!state.mine) { await page.waitForTimeout(25); continue; }
        const before = await page.evaluate(() => document.querySelector('#log').textContent);
        // The board first (lit corners, edges, hexes), then the buttons
        if (await page.locator('.target.corner').count()) { await page.locator('.target.corner').first().click({ force: true }); clicks.corner++; }
        else if (await page.locator('.target.edge').count()) {
          await page.locator('.target.edge').first().click({ force: true });
          clicks.edge++;
          // Where both are allowed the page asks "Path or Boat?": wait for that question or the move
          await page.waitForFunction(b => document.querySelector('#log').textContent !== b
            || [...document.querySelectorAll('#choices button')].some(x => x.textContent === 'Build a Path'), before, { timeout: 5000 });
          const path = page.locator('#choices button', { hasText: 'Build a Path' });
          if (await path.count()) await path.click();
        }
        else if (await page.locator('.target.hex-target').count()) { await page.locator('.target.hex-target').first().click({ force: true }); clicks.hex++; }
        else if (await page.locator('#choices button', { hasText: 'Roll' }).count()) { await page.locator('#choices button', { hasText: 'Roll' }).click(); clicks.roll++; }
        else if (clicks.trade < 2 && await page.locator('#trade-box:not([hidden]) #trade').count()) { await page.click('#trade'); clicks.trade++; }
        else if (await page.locator('#choices button', { hasText: 'End turn' }).count()) { await page.locator('#choices button', { hasText: 'End turn' }).click(); clicks.end++; }
        else if (await page.locator('#choices button').count()) { await page.locator('#choices button').first().click(); clicks.other++; }
        // Each click is a move the server accepted: the log changes
        try {
          await page.waitForFunction(b => document.querySelector('#log').textContent !== b || document.querySelector('#move-error').textContent, before, { timeout: 5000 });
        } catch {
          const seen = await page.evaluate(() => ({
            status: document.querySelector('#status').textContent,
            choices: [...document.querySelectorAll('#choices button')].map(b => b.textContent),
            targets: document.querySelectorAll('.target').length,
            myTurn: document.body.dataset.myTurn,
            phase: document.body.dataset.phase,
            log: [...document.querySelectorAll('#log li')].slice(0, 3).map(l => l.textContent),
          }));
          assert.fail(`no move after a click (clicks so far ${JSON.stringify(clicks)}): ${JSON.stringify(seen)}`);
        }
        assert.equal(await page.textContent('#move-error'), '', 'the server accepted every click');
      }
      assert.equal(clicks.corner, 2, 'two setup Huts');
      assert.ok(clicks.edge >= 2, 'two setup Paths');
      assert.ok(clicks.roll >= 8, `rolled 8 times (${JSON.stringify(clicks)})`);
      assert.ok(await page.locator('.building').count() >= 6, 'everyone\'s setup Huts are drawn');
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
    }
  });

  test(`${name}: a friend's link opens their own seat and the key leaves the address`, async () => {
    const { browser, page, errors } = await open(type);
    try {
      await page.goto(base + '/');
      await page.click('[data-seat="1"][data-kind="person"]');
      await page.fill('input[name="name1"]', 'Matt');
      await page.click('button[type="submit"]');
      await page.waitForSelector('#invites input');
      const link = await page.inputValue('#invites input');
      assert.match(link, /\/\?g=[A-Za-z0-9]{12}#k=/);
      const friend = await (await browser.newContext()).newPage();
      friend.on('pageerror', e => errors.push(e.message));
      await friend.goto(link);
      await friend.waitForSelector('.player strong');
      await friend.waitForFunction(() => [...document.querySelectorAll('.player strong')].some(s => s.textContent === 'Matt (you)'));
      assert.ok(!friend.url().includes('#k='), 'the key is stored, not left in the address');
      // The host sees Matt as online once his page is open
      await page.waitForFunction(() => ![...document.querySelectorAll('.player')].some(p => p.textContent.includes('Matt') && p.textContent.includes('away')));
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
    }
  });

  test(`${name}: pages fit a phone without scrolling sideways`, async () => {
    const { browser, page, errors } = await open(type, { width: 390, height: 844 });
    try {
      for (const path of ['/', '/viewer.html?seed=betta']) {
        await page.goto(base + path);
        await page.waitForLoadState('networkidle');
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 390, path);
      }
      await page.goto(base + '/');
      await page.click('button[type="submit"]');
      await page.waitForSelector('#board svg');
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 390, 'the game');
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
    }
  });
}
