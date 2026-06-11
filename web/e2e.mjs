// Headless browser smoke test of the actual UI.
// Starts a Vite preview server, drives the page with Playwright.
import { chromium } from "playwright";
import { preview } from "vite";

let failures = 0;
const check = (cond, msg) => {
  console.log((cond ? "ok: " : "FAIL: ") + msg);
  if (!cond) failures++;
};

const server = await preview({ preview: { port: 5199 } });
const url = "http://localhost:5199/";

const browser = await chromium.launch();
const page = await browser.newPage();
const errors = [];
const logs = [];
page.on("pageerror", (e) => errors.push("pageerror: " + String(e)));
page.on("console", (m) => {
  logs.push(`[${m.type()}] ${m.text()}`);
  if (m.type() === "error") errors.push(m.text());
});
page.on("worker", (w) => {
  logs.push("worker created: " + w.url());
  w.on("close", () => logs.push("worker closed"));
});
page.on("requestfailed", (r) =>
  logs.push("REQFAIL " + r.url() + " " + (r.failure()?.errorText ?? "")),
);
page.on("response", (r) => {
  if (r.url().includes(".wasm")) logs.push("wasm response " + r.status() + " " + r.url());
});

await page.goto(url, { waitUntil: "networkidle" });

// Board renders 64 squares once the engine has loaded.
await page.waitForSelector(".square");
const squares = await page.locator(".square").count();
check(squares === 64, "board renders 64 squares");
const pieces = await page.locator(".piece").count();
check(pieces === 32, "32 pieces on the start position");
check(
  (await page.locator("#status").textContent())?.includes("White to move"),
  "status shows White to move"
);

// --- Hotseat move: click e2 then e4 ---
const sq = (name) => {
  const file = name.charCodeAt(0) - 97;
  const rank = name.charCodeAt(1) - 49;
  return rank * 8 + file;
};
const clickSquare = async (name) => page.locator(".square").nth(sq(name)).click();

await clickSquare("e2");
const targets = await page.locator(".square.target, .square.capture").count();
check(targets > 0, "selecting e2 highlights legal targets");
await clickSquare("e4");
await page
  .waitForFunction(() => document.querySelector("#moves")?.textContent?.includes("e4"), null, {
    timeout: 4000,
  })
  .catch(() => {});
const movesText = await page.locator("#moves").textContent();
check(movesText?.includes("e4"), "e2-e4 appears in the move list");

// --- Vs computer: switch mode, the engine should reply via the worker ---
await page.selectOption("#mode", "computer");
// New game starts (human = white). Play a move and wait for a black reply.
await clickSquare("e2");
await clickSquare("e4");
let vsText = "";
try {
  await page.waitForFunction(
    () => (document.querySelector("#moves")?.textContent?.trim().split(/\s+/).length ?? 0) >= 2,
    null,
    { timeout: 10000 }
  );
  vsText = (await page.locator("#moves").textContent())?.trim() ?? "";
} catch {
  vsText = (await page.locator("#moves").textContent())?.trim() ?? "";
  console.log("--- diagnostics ---");
  console.log("status:", await page.locator("#status").textContent());
  console.log("io-error:", await page.locator("#io-error").textContent());
  console.log("moves:", JSON.stringify(vsText));
  console.log("logs:\n" + logs.join("\n"));
  console.log("errors:\n" + errors.join("\n"));
}
check(vsText.split(/\s+/).length >= 2, "computer replied with a move");

// --- PGN export stamps today's local date ---
// The Import/Export controls live in a collapsed <details>; open it first.
await page.locator("details.io > summary").click();
await page.locator("#export-pgn").click();
const pgn = await page.locator("#pgn").inputValue();
const dateLine = pgn.split("\n").find((l) => l.startsWith("[Date"));
const today = (() => {
  const d = new Date();
  const p = (x) => String(x).padStart(2, "0");
  return `${d.getFullYear()}.${p(d.getMonth() + 1)}.${p(d.getDate())}`;
})();
check(dateLine === `[Date "${today}"]`, `PGN export carries today's date (got ${dateLine})`);

check(errors.length === 0, "no page/console errors: " + errors.join(" | "));

await browser.close();
await server.httpServer.close();
console.log(failures === 0 ? "\nE2E PASSED" : `\n${failures} E2E CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
