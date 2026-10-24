// Motion smoothness, measured (Task List 1.10): the built screens are served and opened in the
// engine the app really uses on this system, Edge on Windows (WebView2 is Edge's engine) and
// WebKit on a Mac (WKWebView's engine). Every frame is timed through the Wordplay opening and a
// screen change; on Edge the processor is slowed four times, as on a slow laptop. It fails when
// motion stutters: frames this long are what a person sees as a jerk.
//
// node perf/motion-frames.mjs --engine msedge|webkit [--out result.json]
import { spawn } from "node:child_process";
import { writeFileSync } from "node:fs";
import process from "node:process";
import { chromium, webkit } from "playwright";

const arg = (name, fallback) => {
  const at = process.argv.indexOf(name);
  return at > 0 ? process.argv[at + 1] : fallback;
};
const engine = arg("--engine", "msedge");
const out = arg("--out", "motion-frames.json");
const PORT = 4173;
const SLOWDOWN = 4;
// Limits for a smooth feel: most frames inside about two refreshes, none frozen for long.
const P95_MS = 50;
const LONGEST_MS = 250;
const SLOW_SHARE = 0.05;
// Building a new screen happens once, at the click, before any motion: it gets its own limit
// (at four times slower, about a tenth of a second on an ordinary laptop).
const READY_MS = 400;

function serve() {
  const server = spawn(`pnpm exec vite preview --port ${PORT} --strictPort`, { shell: true, stdio: "ignore" });
  return server;
}

async function ready(page) {
  for (let i = 0; i < 60; i++) {
    try {
      await page.goto(`http://localhost:${PORT}/`, { waitUntil: "domcontentloaded" });
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  throw new Error("the built screens were not served");
}

/** Frame times (ms between frames) recorded from now for `ms`. */
async function frames(page, ms) {
  return page.evaluate(
    (ms) =>
      new Promise((done) => {
        const gaps = [];
        let last = performance.now();
        const end = last + ms;
        const tick = (now) => {
          gaps.push(now - last);
          last = now;
          if (now < end) requestAnimationFrame(tick);
          else done(gaps);
        };
        requestAnimationFrame(tick);
      }),
    ms,
  );
}

function summary(name, gaps) {
  const sorted = [...gaps].sort((a, b) => a - b);
  const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? 0;
  const longest = sorted.at(-1) ?? 0;
  const slow = gaps.filter((g) => g > P95_MS).length / Math.max(1, gaps.length);
  const ok = p95 <= P95_MS && longest <= LONGEST_MS && slow <= SLOW_SHARE;
  return { name, frames: gaps.length, p95: Math.round(p95), longest: Math.round(longest), slowShare: Number(slow.toFixed(3)), ok };
}

const server = serve();
let failed = false;
try {
  const browser = engine === "webkit" ? await webkit.launch() : await chromium.launch({ channel: "msedge" });
  const page = await browser.newPage({ viewport: { width: 960, height: 640 } });
  if (engine !== "webkit") {
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: SLOWDOWN });
  }
  // A fresh browser has nothing stored, so this is a first launch: the opening plays.
  await ready(page);
  const opening = summary("the opening", await frames(page, 6500));
  // A screen change, there and back.
  const start = page.getByRole("button", { name: /Start a full move/ }).first();
  await start.waitFor({ timeout: 15000 });
  // Recorded across the click (the click itself waits on the page, so recording after it would
  // miss the frame that builds the new screen). The longest frame in the first 600 ms after the
  // click is that build; every other frame is the motion.
  const CLICK_AT = 300;
  const across = frames(page, 3000);
  await page.waitForTimeout(CLICK_AT);
  await start.click();
  const gaps = await across;
  let t = 0;
  let buildAt = -1;
  gaps.forEach((g, i) => {
    t += g;
    if (t >= CLICK_AT && t <= CLICK_AT + 600 && (buildAt < 0 || g > gaps[buildAt])) buildAt = i;
  });
  const readyMs = Math.round(buildAt >= 0 ? gaps[buildAt] : 0);
  const built = { name: "the new screen ready", frames: 1, p95: readyMs, longest: readyMs, slowShare: 0, ok: readyMs <= READY_MS };
  const screen = summary("the screen change's motion", gaps.filter((_, i) => i !== buildAt));
  const result = { engine, slowedDown: engine === "webkit" ? 1 : SLOWDOWN, limits: { p95: P95_MS, longest: LONGEST_MS, slowShare: SLOW_SHARE, ready: READY_MS }, results: [opening, built, screen] };
  writeFileSync(out, JSON.stringify(result, null, 2));
  for (const r of result.results) {
    console.log(`${r.ok ? "smooth" : "STUTTERS"}: ${r.name} on ${engine}: ${r.frames} frames, 95% within ${r.p95} ms, longest ${r.longest} ms, ${Math.round(r.slowShare * 100)}% slow`);
    if (!r.ok) failed = true;
  }
  await browser.close();
} finally {
  server.kill();
}
process.exit(failed ? 1 : 0);
