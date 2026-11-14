import { describe, expect, it } from "vitest";

import { draftToFilter } from "./filters";

const empty = {
  minDate: "",
  maxDate: "",
  minDistance: "",
  maxDistance: "",
  minDuration: "",
  maxDuration: "",
  minElevation: "",
  maxElevation: "",
};

describe("draftToFilter", () => {
  it("drops empty fields and keeps the search text", () => {
    expect(draftToFilter(empty, "hill")).toEqual({ searchText: "hill" });
  });

  it("converts units and raises a max below its min", () => {
    const filter = draftToFilter({
      ...empty,
      minDistance: "10",
      maxDistance: "5",
      minDuration: "30",
    });
    expect(filter).toEqual({ minDistance: 10, maxDistance: 10, minDuration: 1800 });
  });

  it("uses local midnight and end of day for dates", () => {
    const filter = draftToFilter({ ...empty, minDate: "2025-05-06", maxDate: "2025-05-06" });
    expect(new Date(filter.minDate! * 1000).getHours()).toBe(0);
    expect(new Date(filter.maxDate! * 1000).getHours()).toBe(23);
  });
});
