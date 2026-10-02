// Commit graph lane layout.
//
// Commits arrive in --date-order (children before parents). We keep a list
// of "lanes", each waiting for a particular commit hash. When a commit
// arrives it takes the lane that was waiting for it (or a fresh one if it
// is a branch tip), any other lanes waiting for it converge into it, and
// its parents are then placed into lanes for the rows below.
//
// Each row records the line segments to draw in lane units: x is the lane
// index, y runs 0 (top of row) → 0.5 (the commit dot) → 1 (bottom).

import type { Commit } from "./api";

export interface Segment {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  color: number;
}

export interface GraphRow {
  col: number;
  color: number;
  segments: Segment[];
  lanes: number;
}

export interface GraphLayout {
  rows: GraphRow[];
  maxLanes: number;
}

export const LANE_COLORS = 8;

export function layoutGraph(commits: Commit[]): GraphLayout {
  const lanes: (string | null)[] = [];
  const colors: number[] = [];
  let nextColor = 0;
  let maxLanes = 1;
  const rows: GraphRow[] = [];

  const freeSlot = (avoid = -1) => {
    for (let i = 0; i < lanes.length; i++) if (lanes[i] === null && i !== avoid) return i;
    return lanes.length;
  };

  for (const c of commits) {
    let col = lanes.indexOf(c.hash);
    const isTip = col === -1;
    if (isTip) {
      col = freeSlot();
      lanes[col] = c.hash;
      colors[col] = nextColor++ % LANE_COLORS;
    }
    const before = lanes.slice();
    const beforeColors = colors.slice();
    const color = colors[col];
    const segments: Segment[] = [];

    // Incoming: lanes above that end at this commit, plus lanes passing by.
    before.forEach((h, j) => {
      if (h === null) return;
      if (h === c.hash) {
        if (j === col && isTip) return;
        segments.push({ x1: j, y1: 0, x2: col, y2: 0.5, color: beforeColors[j] });
        if (j !== col) lanes[j] = null;
      } else {
        segments.push({ x1: j, y1: 0, x2: j, y2: 1, color: beforeColors[j] });
      }
    });

    // Outgoing: first parent continues this lane; others branch off.
    if (c.parents.length === 0) {
      lanes[col] = null;
    } else {
      lanes[col] = c.parents[0];
      segments.push({ x1: col, y1: 0.5, x2: col, y2: 1, color });
      for (const p of c.parents.slice(1)) {
        let k = lanes.indexOf(p);
        if (k === -1) {
          k = freeSlot(col);
          lanes[k] = p;
          colors[k] = nextColor++ % LANE_COLORS;
        }
        segments.push({ x1: col, y1: 0.5, x2: k, y2: 1, color: colors[k] });
      }
    }

    while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop();
    const width = Math.max(before.length, lanes.length, col + 1);
    maxLanes = Math.max(maxLanes, width);
    rows.push({ col, color, segments, lanes: width });
  }

  return { rows, maxLanes };
}

/** SVG path for one segment, with smooth curves for lane changes. */
export function segmentPath(s: Segment, laneW: number, rowH: number): string {
  const x1 = s.x1 * laneW + laneW / 2;
  const x2 = s.x2 * laneW + laneW / 2;
  const y1 = s.y1 * rowH;
  const y2 = s.y2 * rowH;
  if (x1 === x2) return `M${x1} ${y1}V${y2}`;
  const my = (y1 + y2) / 2;
  return `M${x1} ${y1}C${x1} ${my} ${x2} ${my} ${x2} ${y2}`;
}
