import { describe, expect, it } from "vitest";

import {
  align,
  applyMoves,
  distribute,
  snapMove,
  unionBox,
  unitsOf,
  type Boxes,
} from "./share-arrange";
import { DEFAULT_SCENE, type Scene } from "./share-scene";

const size = { width: 1000, height: 800 };
const box = (left: number, top: number, width: number, height: number) => ({
  left,
  top,
  width,
  height,
});
const boxes: Boxes = {
  distance: box(100, 600, 100, 50),
  duration: box(300, 620, 200, 50),
  pace: box(800, 640, 100, 50),
  title: box(400, 50, 200, 60),
};

describe("unitsOf", () => {
  it("keeps groups together and everything else on its own", () => {
    expect(unitsOf(["distance", "title", "duration"], [["distance", "duration"]])).toEqual([
      ["distance", "duration"],
      ["title"],
    ]);
  });
});

describe("align", () => {
  it("centres a single element on the image", () => {
    expect(align([["pace"]], boxes, "centre", size)).toEqual({ pace: [-350, 0] });
    expect(align([["title"]], boxes, "middle", size)).toEqual({ title: [0, 320] });
  });

  it("lines several up on the box around them", () => {
    expect(align([["distance"], ["duration"], ["pace"]], boxes, "top", size)).toEqual({
      distance: [0, 0],
      duration: [0, -20],
      pace: [0, -40],
    });
  });

  it("moves a group as one", () => {
    // The group spans 100–500, so its centre goes from 300 to 500.
    expect(align([["distance", "duration"]], boxes, "centre", size)).toEqual({
      distance: [200, 0],
      duration: [200, 0],
    });
  });
});

describe("distribute", () => {
  it("shares the space between the outer two of three", () => {
    // 100–900 holds 400 px of boxes: 200 px between each.
    const moves = distribute([["pace"], ["distance"], ["duration"]], boxes, "horizontal", size);
    expect(moves).toEqual({ distance: [0, 0], duration: [100, 0], pace: [0, 0] });
  });

  it("spaces one or two evenly across the image", () => {
    // 1000 px less 200 px of boxes, in three equal gaps.
    const moves = distribute([["distance"], ["pace"]], boxes, "horizontal", size);
    expect(moves.distance![0]).toBeCloseTo(800 / 3 - 100);
    expect(moves.pace![0]).toBeCloseTo((800 / 3) * 2 + 100 - 800);
  });
});

describe("snapMove", () => {
  it("puts the nearest edge or the centre on a line", () => {
    // Lines every 100 px from the centre (500 × 400). Left edge 130 is nearest to 100.
    expect(snapMove(box(130, 0, 20, 10), 0, 0, size, 100)).toEqual([-30, 0]);
    // The right edge 290 is 10 px from 300; the left edge 235 and centre 262.5 are further.
    expect(snapMove(box(235, 0, 55, 10), 0, 0, size, 100)).toEqual([10, 0]);
    // After moving by 52, the centre is at 497: it snaps to the centre line.
    expect(snapMove(box(380, 0, 130, 10), 52, 0, size, 300)).toEqual([55, 0]);
  });
});

describe("applyMoves", () => {
  it("moves centres by pixels from where the drag started", () => {
    const from: Scene = { ...DEFAULT_SCENE, width: 1000, height: 800 };
    const moved = applyMoves(from, from, { route: [100, -80], title: [-2000, 0] });
    expect(moved.route.x).toBeCloseTo(from.route.x + 0.1);
    expect(moved.route.y).toBeCloseTo(from.route.y - 0.1);
    // Kept on the image.
    expect(moved.info.positions.title.x).toBe(0);
  });
});

describe("unionBox", () => {
  it("ignores hidden elements", () => {
    expect(unionBox(["distance", "calories"], boxes)).toEqual(boxes.distance);
    expect(unionBox(["calories"], boxes)).toBeNull();
  });
});
