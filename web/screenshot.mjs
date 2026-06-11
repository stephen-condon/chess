import { chromium } from "playwright";
import { preview } from "vite";

const server = await preview({ preview: { port: 5198 } });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1100, height: 760 } });
await page.goto("http://localhost:5198/", { waitUntil: "networkidle" });
await page.waitForSelector(".piece");

const sq = (n) => (n.charCodeAt(1) - 49) * 8 + (n.charCodeAt(0) - 97);
const click = (n) => page.locator(".square").nth(sq(n)).click();
for (const [a, b] of [["e2", "e4"], ["e7", "e5"], ["g1", "f3"], ["b8", "c6"], ["f1", "b5"]]) {
  await click(a);
  await click(b);
  await page.waitForTimeout(60);
}
await click("a7"); // select a pawn to show highlighting

await page.screenshot({ path: "screenshot.png" });
await browser.close();
await server.httpServer.close();
console.log("saved screenshot.png");
