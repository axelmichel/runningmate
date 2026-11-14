import { describe, expect, it } from "vitest";

import { VIEW_MODES } from "./view-mode";

describe("VIEW_MODES", () => {
  it("lists the four activity views in sidebar order", () => {
    expect(VIEW_MODES.map((v) => v.mode)).toEqual(["All", "Running", "Cycling", "Walking"]);
  });
});
