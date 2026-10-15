export function relativeTime(unix: number): string {
  const s = Math.floor(Date.now() / 1000) - unix;
  if (s < 60) return "just now";
  const units: [number, string][] = [
    [60, "minute"],
    [3600, "hour"],
    [86400, "day"],
    [604800, "week"],
    [2629800, "month"],
    [31557600, "year"],
  ];
  let i = units.length - 1;
  while (i > 0 && s < units[i][0]) i--;
  const n = Math.floor(s / units[i][0]);
  return `${n} ${units[i][1]}${n === 1 ? "" : "s"} ago`;
}

export function fullDate(unix: number): string {
  return new Date(unix * 1000).toLocaleString();
}

export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return "?";
  return ((parts[0][0] ?? "") + (parts.length > 1 ? parts[parts.length - 1][0] : "")).toUpperCase();
}

export function basename(p: string): string {
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p;
}

export function dirname(p: string): string {
  const parts = p.split("/");
  parts.pop();
  return parts.join("/");
}

export const STATUS_LABEL: Record<string, string> = {
  M: "Modified",
  A: "Added",
  D: "Deleted",
  R: "Renamed",
  C: "Copied",
  T: "Type changed",
  U: "Conflicted",
  "?": "New",
};

/** Guess a sensible folder name from a clone URL. */
export function repoNameFromUrl(url: string): string {
  const cleaned = url.trim().replace(/\/+$/, "").replace(/\.git$/, "");
  return cleaned.split(/[/:]/).pop() ?? "";
}

export function shortHash(h: string): string {
  return h.slice(0, 7);
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

/** ISO 8601 timestamp to unix seconds, or null. */
export function isoToUnix(iso: string | null | undefined): number | null {
  const ms = iso ? Date.parse(iso) : NaN;
  return Number.isNaN(ms) ? null : Math.floor(ms / 1000);
}

const SEMVER_RE = /^(v?)(\d+)\.(\d+)\.(\d+)$/;

/** The next patch, minor and major versions after the highest x.y.z tag, keeping its "v" prefix. */
export function nextVersions(tags: string[]): { patch: string; minor: string; major: string } | null {
  let best: [string, number, number, number] | null = null;
  for (const t of tags) {
    const m = SEMVER_RE.exec(t);
    if (!m) continue;
    const v: [string, number, number, number] = [m[1], +m[2], +m[3], +m[4]];
    if (!best || v[1] > best[1] || (v[1] === best[1] && (v[2] > best[2] || (v[2] === best[2] && v[3] > best[3])))) best = v;
  }
  if (!best) return null;
  const [p, major, minor, patch] = best;
  return { patch: `${p}${major}.${minor}.${patch + 1}`, minor: `${p}${major}.${minor + 1}.0`, major: `${p}${major + 1}.0.0` };
}
