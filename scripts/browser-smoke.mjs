// Development/CI only: the application itself does not require Node or JavaScript.
import { chromium } from 'playwright';
import { mkdir } from 'node:fs/promises';
import assert from 'node:assert/strict';
const output = process.env.HPH_SCREENSHOT_DIR || 'artifacts/browser';
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1050 } });
const page = await context.newPage();
const errors = [];
page.on('pageerror', e => errors.push(e.message));
const base = process.env.HPH_ORIGIN || 'http://127.0.0.1:3000';
const visit = async tab => { await page.goto(`${base}/app?tab=${tab}`); await page.locator('main h1').waitFor(); };
const clickSubmit = async (locator) => { await Promise.all([page.waitForNavigation(), locator.click()]); assert(!await page.getByRole('alert').count(), await page.title()); };
try {
  await page.goto(`${base}/login`);
  await page.screenshot({ path: `${output}/login.png`, fullPage: true });
  await page.getByLabel('Work email').fill('portfolio@example.test');
  await page.getByLabel('Password', { exact: true }).fill('portfolio test password');
  await clickSubmit(page.getByRole('button', { name: 'Sign in' }));
  await clickSubmit(page.getByRole('button', { name: /Studio Aurora/ }));
  await visit('people');
  for (const [name, number, job, department] of [['Elena Conti','E001','People manager','People & Culture'],['Marco Riva','E002','Product designer','Design'],['Sofia Bianchi','E003','Software engineer','Engineering']]) {
    await page.getByLabel('Full name').fill(name);
    await page.getByLabel('Employee number').fill(number);
    await page.getByLabel('Department', { exact: true }).fill(department);
    await page.getByLabel('Job title').fill(job);
    await clickSubmit(page.getByRole('button', { name: 'Save employee' }));
  }
  await visit('overview');
  await page.screenshot({ path: `${output}/workspace-desktop.png`, fullPage: true });
  await visit('attendance');
  await page.screenshot({ path: `${output}/clock-0.png` });
  for (const [i,name] of ['Clock in','Start break','End break','Clock out'].entries()) {
    await page.waitForTimeout(1100); // Server records one event per second per employee.
    await clickSubmit(page.getByRole('button', { name, exact: true }));
    await page.screenshot({ path: `${output}/clock-${i+1}.png` });
  }
  await page.reload();
  assert.equal(await page.locator('tbody tr').count(), 4);
  await visit('settings');
  await page.getByLabel('Team notes', { exact: false }).check();
  await clickSubmit(page.getByRole('button', { name: 'Save modules' }));
  await visit('notes');
  await page.getByLabel('Your note').fill('Welcome to Studio Aurora. A little less admin, a little more room for people.');
  await clickSubmit(page.getByRole('button', { name: 'Post note' }));
  assert(await page.getByText('Welcome to Studio Aurora.', { exact: false }).count());
  await visit('settings');
  await page.getByLabel('Theme', { exact: true }).selectOption('indigo');
  await page.getByLabel('Accent color').fill('#423c69');
  await clickSubmit(page.getByRole('button', { name: 'Save appearance' }));
  await visit('overview');
  await page.screenshot({ path: `${output}/workspace-indigo.png`, fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  for (const tab of ['overview','people','attendance','reports','settings','notes','account']) {
    await visit(tab);
    const fits = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
    assert(fits, `Horizontal overflow on ${tab}`);
  }
  await visit('overview');
  await page.screenshot({ path: `${output}/workspace-mobile.png`, fullPage: true });
  await clickSubmit(page.getByRole('button', { name: 'Sign out', exact: true }));
  await page.goto(`${base}/app`);
  assert(page.url().endsWith('/login'), 'Logout must revoke the session');
  assert.deepEqual(errors, []);
  console.log('PASS: desktop/mobile login, employee creation, clock/break cycle, persistence, modules, notes, themes and logout.');
} finally { await browser.close(); }
