// Runs from `npm version`: copies the new package.json version into
// Cargo.toml and Cargo.lock so the Rust crate matches the app.
// tauri.conf.json reads its version from package.json directly.
import { readFileSync, writeFileSync } from "node:fs";

const { version } = JSON.parse(readFileSync("package.json", "utf8"));

function replace(path, pattern) {
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) throw new Error(`No version found in ${path}`);
  writeFileSync(path, text.replace(pattern, `$1${version}$2`));
}

replace("src-tauri/Cargo.toml", /(\[package\][^[]*?\nversion = ")[^"]+(")/);
replace("src-tauri/Cargo.lock", /(\nname = "gitout"\nversion = ")[^"]+(")/);
console.log(`Synced Cargo version to ${version}`);
