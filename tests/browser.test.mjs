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
        else if (await page.locator('.target.edge').count()) { await page.locator('.target.edge').first().click({ force: true }); clicks.edge++; }
        else if (await page.locator('.target.hex-target').count()) { await page.locator('.target.hex-target').first().click({ force: true }); clicks.hex++; }
        else if (await page.locator('#choices button', { hasText: 'Roll' }).count()) { await page.locator('#choices button', { hasText: 'Roll' }).click(); clicks.roll++; }
        else if (clicks.trade < 2 && await page.locator('#trade-box:not([hidden]) #trade').count()) { await page.click('[data-tab="trade"]'); await page.click('#trade'); clicks.trade++; }
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

  test(`${name}: the Build shop lights only what you pick, and an offer to a bot gets an answer`, async () => {
    const { browser, page, errors } = await open(type);
    try {
      await page.goto(base + '/');
      await page.click('[data-seat="2"][data-kind=""]');   // you and one bot
      await page.click('button[type="submit"]');
      await page.waitForSelector('#board svg');
      // Play by clicking (setup, rolls, squall, ends) until it's our main turn with cards
      for (let i = 0; i < 300; i++) {
        const s = await page.evaluate(() => ({ phase: document.body.dataset.phase, mine: document.body.dataset.myTurn === 'true', cards: [...document.querySelectorAll('#hand .card')].some(c => c.dataset.count !== '0') }));
        if (s.mine && s.phase === 'main' && s.cards) break;
        if (!s.mine) { await page.waitForTimeout(25); continue; }
        const before = await page.evaluate(() => document.querySelector('#log').textContent);
        if (await page.locator('.target.corner').count()) await page.locator('.target.corner').first().click({ force: true });
        else if (await page.locator('.target.edge').count()) {
          await page.locator('.target.edge').first().click({ force: true });
          await page.waitForFunction(b => document.querySelector('#log').textContent !== b || [...document.querySelectorAll('#choices button')].some(x => x.textContent === 'Build a Path'), before);
          if (await page.locator('#choices button', { hasText: 'Build a Path' }).count()) await page.locator('#choices button', { hasText: 'Build a Path' }).click();
        }
        else if (await page.locator('.target.hex-target').count()) await page.locator('.target.hex-target').first().click({ force: true });
        else if (await page.locator('#choices button', { hasText: 'Roll' }).count()) await page.locator('#choices button', { hasText: 'Roll' }).click();
        else if (await page.locator('#choices button', { hasText: 'End turn' }).count()) await page.locator('#choices button', { hasText: 'End turn' }).click();
        else if (await page.locator('#choices button').count()) await page.locator('#choices button').first().click();
        await page.waitForFunction(b => document.querySelector('#log').textContent !== b, before, { timeout: 5000 });
      }
      assert.equal(await page.evaluate(() => document.body.dataset.phase), 'main');
      // The turn bar shows the roll as two dice; every player has a Renown bar
      assert.equal(await page.locator('#dice .die').count(), 2);
      const pips = await page.locator('#dice .pip').count();
      const total = Number(await page.textContent('#dice .total'));
      assert.equal(pips, total, 'the dice faces add up to the total shown');
      assert.equal(await page.locator('#players .bar .fill').count(), 2);

      // The shop: Path, Hut, Lodge; nothing lit until one is picked; each disabled one says why
      assert.deepEqual(await page.locator('#shop [data-build]').evaluateAll(bs => bs.map(b => b.dataset.build)), ['path', 'hut', 'lodge']);
      assert.equal(await page.locator('.target').count(), 0, 'on your main turn nothing is lit until you pick');
      for (const item of await page.locator('#shop .shop-item').all()) {
        const b = item.locator('button');
        if (await b.isDisabled()) assert.notEqual((await item.locator('.why').textContent()).trim(), '', 'a disabled item says why');
      }
      // Project cards are in the shop too: hidden by default, so one card drawn unseen, with its cost
      const card = page.locator('#shop .shop-item.project-item');
      assert.equal(await card.count(), 1);
      assert.match(await card.textContent(), /Project card.*Drawn unseen.*\d+ left/);
      assert.deepEqual(await card.locator('.chip').allTextContents(), ['Grain', 'Copper', 'Fish']);
      const canBuy = await page.evaluate(() => [2, 3, 4].every(r => Number(document.querySelectorAll('#hand .card')[r].dataset.count) >= 1));
      assert.equal(await card.locator('[data-buy="buy 0"]').isEnabled(), canBuy, 'Buy is on exactly when the hand pays for it');
      const ready = page.locator('#shop [data-build]:not([disabled])');
      if (await ready.count()) {
        const kind = await ready.first().getAttribute('data-build');
        await ready.first().click();
        const lit = await page.locator('.target').count();
        const ofKind = await page.locator(kind === 'hut' || kind === 'lodge' ? '.target.corner' : '.target.edge').count();
        assert.ok(lit > 0 && lit === ofKind, `only ${kind} spots are lit`);
        await page.locator(`#shop [data-build="${kind}"]`).click();   // now "Cancel"
        assert.equal(await page.locator('.target').count(), 0);
      }

      // An offer: one card we hold for one of another resource, in the Trade tab
      await page.click('[data-tab="trade"]');
      await page.waitForSelector('#deal-box:not([hidden])');
      const giveIndex = await page.evaluate(() => [...document.querySelectorAll('#deal-give .stepper')].findIndex(s => !s.querySelector('button:last-child').disabled));
      await page.locator('#deal-give .stepper').nth(giveIndex).locator('button').last().click();
      await page.locator('#deal-get .stepper').nth((giveIndex + 1) % 5).locator('button').last().click();
      await page.click('#make-offer');
      await page.waitForSelector('#offer-box:not([hidden])');
      await page.waitForFunction(() => !document.querySelector('#offer-answers').textContent.includes('thinking'), null, { timeout: 5000 });
      const answer = await page.textContent('#offer-answers');
      assert.match(answer, /accepted|declined/);
      if (/accepted/.test(answer)) await page.locator('#offer-buttons button', { hasText: 'Trade with' }).click();
      else await page.locator('#offer-buttons button', { hasText: 'Withdraw' }).click();
      await page.waitForSelector('#offer-box[hidden]', { state: 'attached' });
      assert.match(await page.textContent('#log'), /offered/);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
    }
  });

  test(`${name}: with projects face up, the shop shows each face-up card by name`, async () => {
    const { browser, page } = await open(type);
    try {
      await page.goto(base + '/');
      await page.check('input[name="faceUp"]');
      await page.click('button[type="submit"]');
      await page.waitForSelector('#shop .project-item');
      const names = await page.locator('#shop .project-item strong').allTextContents();
      assert.equal(names.length, 3, 'the face-up row has 3 cards');
      for (const n of names) assert.ok(['Pilot', 'Lighthouse', 'Road Crew', 'Good Harvest', 'Good Bargain'].includes(n), n);
      assert.deepEqual(await page.locator('#shop [data-buy]').evaluateAll(bs => bs.map(b => b.dataset.buy)), ['buy 0', 'buy 1', 'buy 2']);
      assert.ok(await page.locator('#shop [data-buy]').first().isDisabled(), 'not buyable during setup');
    } finally {
      await browser.close();
    }
  });

  test(`${name}: a Lodge spot can be clicked anywhere on it, including the Hut in its middle`, async () => {
    const { browser, page } = await open(type);
    try {
      await page.goto(base + '/');
      await page.click('button[type="submit"]');
      await page.waitForSelector('#board svg');
      // A Lodge target as the page draws one, over a Hut, at the first corner of the board
      const hit = await page.evaluate(() => {
        const svg = document.querySelector('#board svg');
        const NS = 'http://www.w3.org/2000/svg';
        const g = svg.lastElementChild;   // the targets layer
        const hut = document.createElementNS(NS, 'circle');
        hut.setAttribute('class', 'building hut p0');
        const ring = document.createElementNS(NS, 'circle');
        ring.setAttribute('class', 'target corner lodge');
        for (const c of [hut, ring]) { c.setAttribute('cx', 0); c.setAttribute('cy', 0); }
        hut.setAttribute('r', 7.5);
        ring.setAttribute('r', 13);
        g.append(hut, ring);
        const box = ring.getBoundingClientRect();
        const at = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
        return at === ring;
      });
      assert.ok(hit, 'a click on the middle of a Lodge spot reaches it');
    } finally {
      await browser.close();
    }
  });

  test(`${name}: a friend sends a counter-offer and the host takes it, all by clicking`, async () => {
    const { browser, page: host, errors } = await open(type);
    try {
      await host.goto(base + '/');
      await host.fill('input[name="name"]', 'Ann');
      await host.click('[data-seat="1"][data-kind="person"]');
      await host.fill('input[name="name1"]', 'Matt');
      await host.click('[data-seat="2"][data-kind=""]');
      await host.click('button[type="submit"]');
      await host.waitForSelector('#invites input');
      const link = await host.inputValue('#invites input');
      const friend = await (await browser.newContext({ viewport: { width: 1200, height: 900 } })).newPage();
      friend.on('pageerror', e => errors.push(e.message));
      await friend.goto(link);
      await friend.waitForSelector('#board svg');
      // One step for whichever page has the move: setup spots, then Roll / End turn
      const step = async page => {
        if (await page.evaluate(() => document.body.dataset.myTurn !== 'true')) return false;
        const before = await page.evaluate(() => document.querySelector('#log').textContent);
        if (await page.locator('.target.corner').count()) await page.locator('.target.corner').first().click({ force: true });
        else if (await page.locator('.target.edge').count()) {
          await page.locator('.target.edge').first().click({ force: true });
          await page.waitForFunction(b => document.querySelector('#log').textContent !== b || [...document.querySelectorAll('#choices button')].some(x => x.textContent === 'Build a Path'), before);
          if (await page.locator('#choices button', { hasText: 'Build a Path' }).count()) await page.locator('#choices button', { hasText: 'Build a Path' }).click();
        }
        else if (await page.locator('.target.hex-target').count()) await page.locator('.target.hex-target').first().click({ force: true });
        else if (await page.locator('#choices button', { hasText: 'Roll' }).count()) await page.locator('#choices button', { hasText: 'Roll' }).click();
        else if (await page.locator('#choices button', { hasText: 'End turn' }).count()) await page.locator('#choices button', { hasText: 'End turn' }).click();
        else if (await page.locator('#choices button').count()) await page.locator('#choices button').first().click();
        await page.waitForFunction(b => document.querySelector('#log').textContent !== b, before, { timeout: 5000 });
        return true;
      };
      // Ready: Ann's main turn, and she holds a resource while Matt holds a different one (so
      // Matt has something else to give back). Returns that resource of Ann's, or -1.
      const counts = page => page.evaluate(() => [...document.querySelectorAll('#hand .card')].map(c => Number(c.dataset.count)));
      const ready = async () => {
        if (!(await host.evaluate(() => document.body.dataset.phase === 'main' && document.body.dataset.myTurn === 'true'))) return -1;
        const [ann, matt] = [await counts(host), await counts(friend)];
        return ann.findIndex((n, r) => n > 0 && matt.some((m, s) => m > 0 && s !== r));
      };
      let gi = -1;
      for (let i = 0; i < 600 && (gi = await ready()) < 0; i++) {
        if (!(await step(host)) && !(await step(friend))) await host.waitForTimeout(25);
      }
      assert.ok(gi >= 0, 'reached a main turn where Ann and Matt hold different resources');

      // Ann offers 1 of that resource for 1 of the next
      await host.click('[data-tab="trade"]');
      await host.locator('#deal-give .stepper').nth(gi).locator('button').last().click();
      await host.locator('#deal-get .stepper').nth((gi + 1) % 5).locator('button').last().click();
      await host.click('#make-offer');

      // Matt's Trade tab opens by itself; he counters: asks for the same card, gives one he has
      await friend.waitForSelector('#offer-box:not([hidden])');
      await friend.locator('#offer-buttons button', { hasText: 'Counter' }).click();
      await friend.waitForSelector('#counter-box:not([hidden])');
      // Clear what he'd give, then give one card he holds that isn't what he'd get
      for (const s of await friend.locator('#counter-mine .stepper').all()) {
        while (!(await s.locator('button').first().isDisabled())) await s.locator('button').first().click();
      }
      const mine = await friend.evaluate(gi => [...document.querySelectorAll('#counter-mine .stepper')].findIndex((s, r) => r !== gi && !s.querySelector('button:last-child').disabled), gi);
      assert.ok(mine >= 0, 'Matt holds a card to give');
      await friend.locator('#counter-mine .stepper').nth(mine).locator('button').last().click();
      await friend.click('#send-counter');

      // Ann sees it and takes it
      await host.waitForSelector('#offer-buttons button:has-text("Take Matt\'s counter")');
      assert.match(await host.textContent('#offer-answers'), /Matt counters: you give/);
      await host.click('#offer-buttons button:has-text("Take Matt\'s counter")');
      await host.waitForSelector('#offer-box[hidden]', { state: 'attached' });
      await friend.waitForFunction(() => document.querySelector('#log').textContent.includes('took your counter-offer'));
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
    }
  });

  test(`${name}: the hand limit, round limit and project visibility set on the New game screen are the game's`, async () => {
    const { browser, page, errors } = await open(type);
    try {
      await page.goto(base + '/');
      await page.fill('input[name="over"]', '9');
      await page.fill('input[name="rounds"]', '15');
      assert.equal(await page.isChecked('input[name="faceUp"]'), false, 'projects are hidden unless you choose face up');
      await page.check('input[name="faceUp"]');
      await page.click('button[type="submit"]');
      await page.waitForSelector('#board svg');
      await page.waitForFunction(() => document.querySelector('#hand-limit').textContent.includes('more than 9 cards'));
      await page.waitForFunction(() => /^(15 rounds|Round \d+ of 15)$/.test(document.querySelector('#dice .round')?.textContent ?? ''));
      await page.click('[data-tab="projects"]');
      assert.equal(await page.locator('#row .project').count(), 3, 'face up: a row of 3');
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
