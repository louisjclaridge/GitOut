// Stamps a release version into the app before building:
//   node scripts/sync-version.mjs 0.3.0
// The release workflow runs this with the version from the GitHub Release
// tag, so versions are never bumped or committed by hand. Writes package.json
// (which tauri.conf.json reads its version from), Cargo.toml and Cargo.lock.
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2]?.replace(/^v/, "");
// Plain X.Y.Z only: the Windows MSI bundler rejects most pre-release suffixes.
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`Expected a version like 1.2.3, got "${process.argv[2] ?? ""}"`);
  process.exit(1);
}

function replace(path, pattern) {
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) throw new Error(`No version found in ${path}`);
  writeFileSync(path, text.replace(pattern, `$1${version}$2`));
}

// Windows checkouts have CRLF line endings, hence \r?\n.
replace("package.json", /(\r?\n  "version": ")[^"]+(")/);
replace("src-tauri/Cargo.toml", /(\[package\][^[]*?\r?\nversion = ")[^"]+(")/);
replace("src-tauri/Cargo.lock", /(\r?\nname = "gitout"\r?\nversion = ")[^"]+(")/);
console.log(`Set app version to ${version}`);
