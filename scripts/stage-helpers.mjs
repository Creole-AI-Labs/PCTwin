// Builds the helper programs the app ships with (pctwin-helper, pctwin-handover) and stages them
// where the installer settings (app/src-tauri/bundle/*.json) take them from:
//   app/src-tauri/binaries/<name>-<target triple>[.exe]   Tauri's external binaries (Windows, Mac):
//                                                          installed beside the app
//   app/src-tauri/binaries/plain/<name>                   Linux: the package puts them in
//                                                          /usr/lib/pctwin, as the polkit policy says
// Run by `tauri build` before the app is built (beforeBuildCommand, with --config bundle/<os>.json).
// Usage: node scripts/stage-helpers.mjs [debug|release]   (default: release)
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const mode = process.argv[2] ?? "release";
if (mode !== "debug" && mode !== "release") throw new Error(`debug or release, not ${mode}`);

const cargo = (args) => execFileSync("cargo", args, { cwd: root, stdio: "inherit" });
const build = ["build", "--locked", "-p", "pctwin-helper", "-p", "pctwin-move", "--bin", "pctwin-helper", "--bin", "pctwin-handover"];
cargo(mode === "release" ? [...build, "--release"] : build);

const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(/^host: (\S+)$/m)?.[1];
if (!host) throw new Error("rustc did not name its host");
const target = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(root, "target");
const exe = process.platform === "win32" ? ".exe" : "";
const out = join(root, "app", "src-tauri", "binaries");
mkdirSync(join(out, "plain"), { recursive: true });
for (const name of ["pctwin-helper", "pctwin-handover"]) {
  const built = join(target, mode, name + exe);
  copyFileSync(built, join(out, `${name}-${host}${exe}`));
  copyFileSync(built, join(out, "plain", name + exe));
  console.log(`staged ${name} (${host}, ${mode})`);
}
