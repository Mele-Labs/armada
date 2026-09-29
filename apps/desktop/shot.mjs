import { chromium } from "playwright";
const [url, out, w = "1440", h = "980", click] = process.argv.slice(2);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: +w, height: +h }, deviceScaleFactor: 2 });
await page.goto(url, { waitUntil: "networkidle" });
await page.waitForTimeout(1500);
for (let i = 0; i < 4; i++) {
  const close = page.getByRole("button", { name: "Close", exact: true });
  if (await close.count() === 0) break;
  await close.first().click().catch(() => {});
  await page.waitForTimeout(400);
}
if (click) { await page.getByRole("tab", { name: new RegExp(click) }).first().click(); await page.waitForTimeout(900); }
await page.waitForTimeout(1200);
await page.screenshot({ path: out });
await browser.close();
