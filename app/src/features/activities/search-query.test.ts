import { describe, expect, it } from "vitest";

import { parseSearch, withSearch } from "./search-query";

describe("parseSearch", () => {
  it("keeps plain words as text", () => {
    expect(parseSearch("  Freiburg   morning ")).toEqual({ text: "Freiburg morning" });
    // Bare numbers aren't distances.
    expect(parseSearch("Marathon 2024 5")).toEqual({ text: "Marathon 2024 5" });
  });

  it("reads about a distance", () => {
    expect(parseSearch("10km")).toEqual({ text: "", minDistance: 9.5, maxDistance: 10.5 });
    expect(parseSearch("Wiehre 10 KM")).toEqual({
      text: "Wiehre",
      minDistance: 9.5,
      maxDistance: 10.5,
    });
    expect(parseSearch("21,1km")).toEqual({ text: "", minDistance: 20.045, maxDistance: 22.155 });
  });

  it("reads at least and at most", () => {
    expect(parseSearch(">21")).toEqual({ text: "", minDistance: 21 });
    expect(parseSearch(">= 10 km trail")).toEqual({ text: "trail", minDistance: 10 });
    expect(parseSearch("≥7.5")).toEqual({ text: "", minDistance: 7.5 });
    expect(parseSearch("<5km")).toEqual({ text: "", maxDistance: 5 });
    expect(parseSearch("<= 5,5")).toEqual({ text: "", maxDistance: 5.5 });
  });

  it("reads ranges", () => {
    expect(parseSearch("10-15km")).toEqual({ text: "", minDistance: 10, maxDistance: 15 });
    expect(parseSearch("10 – 15 km hills")).toEqual({
      text: "hills",
      minDistance: 10,
      maxDistance: 15,
    });
    expect(parseSearch("15km-10km")).toEqual({ text: "", minDistance: 10, maxDistance: 15 });
    // Without a unit it isn't a distance.
    expect(parseSearch("5-4-3 intervals")).toEqual({ text: "5-4-3 intervals" });
  });

  it("narrows several shortcuts down", () => {
    expect(parseSearch(">10 <15 lake")).toEqual({ text: "lake", minDistance: 10, maxDistance: 15 });
    expect(parseSearch(">10 >12")).toEqual({ text: "", minDistance: 12 });
  });

  it("leaves look-alikes as text", () => {
    expect(parseSearch("10kmh -5km 5k")).toEqual({ text: "10kmh -5km 5k" });
  });
});

describe("withSearch", () => {
  it("passes filters without search text through", () => {
    expect(withSearch({ minDistance: 5 })).toEqual({ minDistance: 5 });
  });

  it("splits the search box into text and distance", () => {
    expect(withSearch({ searchText: "Freiburg >21", minDate: 100 })).toEqual({
      searchText: "Freiburg",
      minDate: 100,
      minDistance: 21,
    });
    expect(withSearch({ searchText: "10-15km" })).toEqual({ minDistance: 10, maxDistance: 15 });
  });

  it("intersects with the dialog's distance range", () => {
    expect(withSearch({ searchText: "<15km", minDistance: 5, maxDistance: 20 })).toEqual({
      minDistance: 5,
      maxDistance: 15,
    });
    expect(withSearch({ searchText: ">10", minDistance: 12 })).toEqual({ minDistance: 12 });
  });
});
