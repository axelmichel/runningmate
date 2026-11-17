import { describe, expect, it } from "vitest";

import type { SystemSettings } from "@/bindings/SystemSettings";

import { hiddenBoxes, withBox } from "./panel-boxes";

const none = { running: [], cycling: [], walking: [] };

describe("panel boxes", () => {
  it("hides boxes per sport and shows everything for other activities", () => {
    const settings = {
      activity_panel_hidden: { ...none, cycling: ["weather"] },
    } as unknown as SystemSettings;
    expect(hiddenBoxes(settings, "Cycling")).toEqual(["weather"]);
    expect(hiddenBoxes(settings, "Running")).toEqual([]);
    expect(hiddenBoxes(settings, "All")).toEqual([]);
    expect(hiddenBoxes(undefined, "Cycling")).toEqual([]);
  });

  it("toggles one box of one sport", () => {
    const hidden = withBox(none, "Walking", "map", false);
    expect(hidden).toEqual({ ...none, walking: ["map"] });
    expect(withBox(hidden, "Walking", "map", false)).toEqual(hidden);
    expect(withBox(hidden, "Walking", "map", true)).toEqual(none);
  });
});
