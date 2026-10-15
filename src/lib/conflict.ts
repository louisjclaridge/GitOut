// Parsing a conflicted file's markers into common text and conflict blocks,
// and putting it back together once some or all blocks are resolved.
//
// Lines keep their own line endings, so a file reassembled with nothing
// changed is byte-for-byte what git wrote, CRLF and all.

export interface Common {
  kind: "common";
  lines: string[];
}

export interface Conflict {
  kind: "conflict";
  ours: string[];
  /** Lines from the common ancestor; only present with diff3/zdiff3 markers. */
  base: string[] | null;
  theirs: string[];
  /** The marker lines exactly as written, to restore unresolved blocks. */
  markers: { start: string; base: string | null; mid: string; end: string };
}

export type Segment = Common | Conflict;

export interface Parsed {
  segments: Segment[];
  /** Line ending to use for text typed into the editor. */
  eol: string;
  conflicts: number;
}

export type Pick = "ours" | "theirs" | "ours-theirs" | "theirs-ours" | "base" | "none" | "custom";

export interface Resolution {
  pick: Pick;
  /** The result as edited, for "custom". */
  text: string;
}

const START = /^<{7}(?: |\r?\n|$)/;
const BASE = /^\|{7}(?: |\r?\n|$)/;
const MID = /^={7}(?:\r?\n|$)/;
const END = /^>{7}(?: |\r?\n|$)/;

/** Split into lines that keep their terminators. */
export function splitLines(text: string): string[] {
  return text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
}

/** A line without its terminator, for display. */
export function bare(line: string): string {
  return line.replace(/\r?\n$/, "");
}

/** Null when the markers are unbalanced, so the file can't be split safely. */
export function parseConflicts(text: string): Parsed | null {
  const segments: Segment[] = [];
  const eol = text.includes("\r\n") ? "\r\n" : "\n";
  let common: string[] = [];
  let cur: Conflict | null = null;
  let part: "ours" | "base" | "theirs" = "ours";
  let conflicts = 0;

  for (const line of splitLines(text)) {
    if (!cur) {
      if (START.test(line)) {
        if (common.length) segments.push({ kind: "common", lines: common });
        common = [];
        cur = { kind: "conflict", ours: [], base: null, theirs: [], markers: { start: line, base: null, mid: "", end: "" } };
        part = "ours";
      } else {
        common.push(line);
      }
      continue;
    }
    if (START.test(line)) return null; // nested markers
    if (part === "ours" && BASE.test(line)) {
      cur.base = [];
      cur.markers.base = line;
      part = "base";
    } else if (part !== "theirs" && MID.test(line)) {
      cur.markers.mid = line;
      part = "theirs";
    } else if (part === "theirs" && END.test(line)) {
      cur.markers.end = line;
      segments.push(cur);
      conflicts++;
      cur = null;
    } else if (part === "base") {
      cur.base!.push(line);
    } else {
      cur[part].push(line);
    }
  }
  if (cur) return null;
  if (common.length) segments.push({ kind: "common", lines: common });
  return { segments, eol, conflicts };
}

/** Lines a resolution produces. */
export function resolvedLines(c: Conflict, r: Resolution, eol: string): string[] {
  switch (r.pick) {
    case "ours":
      return c.ours;
    case "theirs":
      return c.theirs;
    case "ours-theirs":
      return [...withEol(c.ours, eol), ...c.theirs];
    case "theirs-ours":
      return [...withEol(c.theirs, eol), ...c.ours];
    case "base":
      return c.base ?? [];
    case "none":
      return [];
    case "custom":
      return r.text === "" ? [] : splitLines(r.text.replace(/\r?\n/g, eol) + eol);
  }
}

/** Ensure the last line ends with a newline, so another block can follow it. */
function withEol(lines: string[], eol: string): string[] {
  if (!lines.length || /\n$/.test(lines[lines.length - 1])) return lines;
  return [...lines.slice(0, -1), lines[lines.length - 1] + eol];
}

/** The text a resolution produces, as shown and edited in the result column. */
export function resolvedText(c: Conflict, r: Resolution, eol: string): string {
  return r.pick === "custom" ? r.text : resolvedLines(c, r, eol).map(bare).join("\n");
}

/** The whole file: resolved blocks replaced, unresolved ones written back with their markers. */
export function assemble(p: Parsed, resolutions: (Resolution | null)[]): string {
  let out = "";
  let i = 0;
  for (const s of p.segments) {
    if (s.kind === "common") {
      out += s.lines.join("");
      continue;
    }
    const r = resolutions[i++];
    if (r) {
      out += resolvedLines(s, r, p.eol).join("");
    } else {
      out += s.markers.start + s.ours.join("");
      if (s.base && s.markers.base) out += s.markers.base + s.base.join("");
      out += s.markers.mid + s.theirs.join("") + s.markers.end;
    }
  }
  return out;
}
