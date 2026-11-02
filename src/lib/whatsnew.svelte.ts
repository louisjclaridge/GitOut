// "What's new": the release notes for this version, shown once after an update.
// The release workflow bakes the GitHub Release's notes into src/whats-new.md.
import { getVersion } from "@tauri-apps/api/app";
import raw from "../whats-new.md?raw";
import { app, updateSettings } from "./state.svelte";

/** The notes to show: the "## What's new" section if there is one, else everything. Empty in dev builds. */
export function releaseNotes(md = raw): string {
  const text = md.replace(/<!--[\s\S]*?-->/g, "").replace(/\r\n/g, "\n").trim();
  const lines = text.split("\n");
  const start = lines.findIndex((l) => /^#{1,3}\s+what['’]?s new\b/i.test(l.trim()));
  if (start === -1) return text;
  const level = lines[start].match(/^#+/)![0].length;
  const end = lines.findIndex((l, i) => i > start && new RegExp(`^#{1,${level}}\\s`).test(l.trim()));
  return lines
    .slice(start + 1, end === -1 ? undefined : end)
    .join("\n")
    .trim();
}

export const whatsNew = $state({ open: false, version: "" });

/**
 * Show the notes if the app was updated since they were last shown. A fresh
 * install just records its version, unless settings show the app was used
 * before (an update from a version that didn't track this yet).
 */
export async function showWhatsNewIfUpdated() {
  const s = app.settings;
  if (!s) return;
  const version = await getVersion().catch(() => "");
  if (!version || s.lastSeenVersion === version) return;
  const updated = s.lastSeenVersion !== null || s.recentRepos.length > 0 || s.accounts.length > 0;
  const forced = import.meta.env.DEV && new URLSearchParams(location.search).has("whatsnew");
  // Recorded before showing, so a crash while it's open doesn't show it again forever.
  if (!forced) await updateSettings({ lastSeenVersion: version });
  if ((updated && releaseNotes()) || forced) Object.assign(whatsNew, { open: true, version });
}

export async function showWhatsNew() {
  whatsNew.version = await getVersion().catch(() => "");
  whatsNew.open = true;
}

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

function inline(s: string): string {
  return escape(s)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>")
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, '$1<a href="$2">$2</a>');
}

/**
 * The small Markdown subset release notes use (headings, lists, paragraphs,
 * bold, italic, code, links) as HTML. Text is escaped first; links are only http(s).
 */
export function renderNotes(md: string): string {
  const out: string[] = [];
  let list: string[] = [];
  let para: string[] = [];
  const flush = () => {
    if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`);
    if (list.length) out.push(`<ul>${list.map((li) => `<li>${inline(li)}</li>`).join("")}</ul>`);
    para = [];
    list = [];
  };
  for (const line of md.split("\n")) {
    const t = line.trim();
    const h = /^(#{1,6})\s+(.*)$/.exec(t);
    const li = /^[-*+]\s+(.*)$/.exec(t);
    if (!t) flush();
    else if (h) {
      flush();
      out.push(`<h3>${inline(h[2])}</h3>`);
    } else if (li) {
      if (para.length) flush();
      list.push(li[1]);
    } else if (list.length && /^\s/.test(line)) list[list.length - 1] += " " + t;
    else {
      if (list.length) flush();
      para.push(t);
    }
  }
  flush();
  return out.join("");
}
