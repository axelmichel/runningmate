import type { Box, Layout } from "./share-render";
import { gridLines, type ElementId, type Scene } from "./share-scene";

/** Where each shown element is on the image, in pixels. */
export type Boxes = Layout;
/** How far to move elements, in pixels. */
export type Moves = Partial<Record<ElementId, [number, number]>>;

export type Alignment = "left" | "centre" | "right" | "top" | "middle" | "bottom";
export type Axis = "horizontal" | "vertical";

/** The element with the rest of its group, or on its own. */
export function groupOf(id: ElementId, groups: ElementId[][]): ElementId[] {
  return groups.find((group) => group.includes(id)) ?? [id];
}

/**
 * The selection as units that move together: each group with its selected members, every
 * other element on its own.
 */
export function unitsOf(selection: ElementId[], groups: ElementId[][]): ElementId[][] {
  const units: ElementId[][] = [];
  const seen = new Set<ElementId>();
  for (const id of selection) {
    if (seen.has(id)) continue;
    const unit = groupOf(id, groups).filter((member) => selection.includes(member));
    unit.forEach((member) => seen.add(member));
    units.push(unit);
  }
  return units;
}

/** The box around all of `ids` that are shown. */
export function unionBox(ids: ElementId[], boxes: Boxes): Box | null {
  const shown = ids.map((id) => boxes[id]).filter((box): box is Box => box !== undefined);
  if (!shown.length) return null;
  const left = Math.min(...shown.map((b) => b.left));
  const top = Math.min(...shown.map((b) => b.top));
  const right = Math.max(...shown.map((b) => b.left + b.width));
  const bottom = Math.max(...shown.map((b) => b.top + b.height));
  return { left, top, width: right - left, height: bottom - top };
}

function moveUnit(moves: Moves, unit: ElementId[], dx: number, dy: number) {
  for (const id of unit) moves[id] = [dx, dy];
}

/**
 * Lines up the units: one unit on the image (e.g. "centre" centres it horizontally), several
 * with each other, on the edges or centre of the box around them all.
 */
export function align(
  units: ElementId[][],
  boxes: Boxes,
  alignment: Alignment,
  size: { width: number; height: number },
): Moves {
  const placed = units
    .map((unit) => ({ unit, box: unionBox(unit, boxes) }))
    .filter((u): u is { unit: ElementId[]; box: Box } => u.box !== null);
  const target =
    placed.length === 1
      ? { left: 0, top: 0, width: size.width, height: size.height }
      : unionBox(units.flat(), boxes);
  const moves: Moves = {};
  if (!target) return moves;
  for (const { unit, box } of placed) {
    const dx =
      alignment === "left"
        ? target.left - box.left
        : alignment === "centre"
          ? target.left + target.width / 2 - (box.left + box.width / 2)
          : alignment === "right"
            ? target.left + target.width - (box.left + box.width)
            : 0;
    const dy =
      alignment === "top"
        ? target.top - box.top
        : alignment === "middle"
          ? target.top + target.height / 2 - (box.top + box.height / 2)
          : alignment === "bottom"
            ? target.top + target.height - (box.top + box.height)
            : 0;
    moveUnit(moves, unit, dx, dy);
  }
  return moves;
}

/**
 * Equal space between the units along an axis. Three or more keep the outer two in place and
 * share the space between them; one or two share the image's width (or height), with the
 * same space at its edges.
 */
export function distribute(
  units: ElementId[][],
  boxes: Boxes,
  axis: Axis,
  size: { width: number; height: number },
): Moves {
  const horizontal = axis === "horizontal";
  const start = (b: Box) => (horizontal ? b.left : b.top);
  const length = (b: Box) => (horizontal ? b.width : b.height);
  const placed = units
    .map((unit) => ({ unit, box: unionBox(unit, boxes) }))
    .filter((u): u is { unit: ElementId[]; box: Box } => u.box !== null)
    .sort((a, b) => start(a.box) + length(a.box) / 2 - (start(b.box) + length(b.box) / 2));
  const moves: Moves = {};
  if (!placed.length) return moves;
  const total = placed.reduce((sum, u) => sum + length(u.box), 0);
  const within = placed.length >= 3;
  const first = within ? start(placed[0].box) : 0;
  const last = placed[placed.length - 1].box;
  const end = within ? start(last) + length(last) : horizontal ? size.width : size.height;
  const gap = (end - first - total) / (within ? placed.length - 1 : placed.length + 1);
  let at = within ? first : first + gap;
  for (const { unit, box } of placed) {
    const shift = at - start(box);
    moveUnit(moves, unit, horizontal ? shift : 0, horizontal ? 0 : shift);
    at += length(box) + gap;
  }
  return moves;
}

/** Nearest of `lines` to `value`. */
function nearest(value: number, lines: number[]): number {
  return lines.reduce((best, line) =>
    Math.abs(line - value) < Math.abs(best - value) ? line : best,
  );
}

/**
 * Adjusts a move so that the moved box sits on the grid: whichever of its left edge, centre
 * and right edge (and top, middle and bottom) is closest to a line, or to the image's edges,
 * lands on it.
 */
export function snapMove(
  box: Box,
  dx: number,
  dy: number,
  size: { width: number; height: number },
  step: number,
): [number, number] {
  const snap = (start: number, length: number, side: number) => {
    const lines = [0, side, ...gridLines(side, step)];
    const offsets = [start, start + length / 2, start + length].map(
      (anchor) => nearest(anchor, lines) - anchor,
    );
    return offsets.reduce((best, offset) => (Math.abs(offset) < Math.abs(best) ? offset : best));
  };
  return [
    dx + snap(box.left + dx, box.width, size.width),
    dy + snap(box.top + dy, box.height, size.height),
  ];
}

const clamp = (value: number) => Math.min(1, Math.max(0, value));

/**
 * The scene with elements moved from where they are in `from` (the positions when a drag or an
 * arrangement started), keeping their centres on the image.
 */
export function applyMoves(scene: Scene, from: Scene, moves: Moves): Scene {
  const positions = { ...scene.info.positions };
  let route = scene.route;
  for (const [id, [dx, dy]] of Object.entries(moves) as [ElementId, [number, number]][]) {
    const origin = id === "route" ? from.route : from.info.positions[id];
    const point = {
      x: clamp(origin.x + dx / scene.width),
      y: clamp(origin.y + dy / scene.height),
    };
    if (id === "route") route = { ...route, ...point };
    else positions[id] = point;
  }
  return { ...scene, route, info: { ...scene.info, positions } };
}
