// Unified diff parsing, plus building single-hunk patches for `git apply`.

export interface DiffLine {
  kind: "add" | "del" | "ctx" | "meta";
  text: string;
  oldNo: number | null;
  newNo: number | null;
}

export interface Hunk {
  header: string;
  lines: DiffLine[];
  /** Raw lines of the hunk including its @@ header, for patch building. */
  raw: string[];
}

export interface ParsedDiff {
  /** File header lines (diff --git, index, ---, +++). */
  header: string[];
  hunks: Hunk[];
  binary: boolean;
}

const HUNK_RE = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

export function parseDiff(text: string): ParsedDiff {
  const out: ParsedDiff = { header: [], hunks: [], binary: false };
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  let hunk: Hunk | null = null;
  let oldNo = 0;
  let newNo = 0;
  for (const line of lines) {
    const m = HUNK_RE.exec(line);
    if (m) {
      oldNo = +m[1];
      newNo = +m[2];
      hunk = { header: line, lines: [], raw: [line] };
      out.hunks.push(hunk);
      continue;
    }
    if (!hunk) {
      if (line.startsWith("Binary files") || line === "GIT binary patch") out.binary = true;
      out.header.push(line);
      continue;
    }
    hunk.raw.push(line);
    const c = line[0];
    const body = line.slice(1);
    if (c === "+") hunk.lines.push({ kind: "add", text: body, oldNo: null, newNo: newNo++ });
    else if (c === "-") hunk.lines.push({ kind: "del", text: body, oldNo: oldNo++, newNo: null });
    else if (c === "\\") hunk.lines.push({ kind: "meta", text: line, oldNo: null, newNo: null });
    else hunk.lines.push({ kind: "ctx", text: body, oldNo: oldNo++, newNo: newNo++ });
  }
  return out;
}

/** A complete patch containing only one hunk of the file. */
export function hunkPatch(diff: ParsedDiff, hunk: Hunk): string {
  return [...diff.header, ...hunk.raw].join("\n") + "\n";
}
