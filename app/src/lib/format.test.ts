import { describe, expect, it } from "vitest";

import { formatDuration, formatHours, formatMinSec, formatPace, parseDuration } from "./format";

describe("format", () => {
  it("formats durations", () => {
    expect(formatDuration(2932)).toBe("0:48:52");
    expect(formatDuration(36000)).toBe("10:00:00");
    expect(formatDuration(null)).toBe("–");
  });

  it("parses durations", () => {
    expect(parseDuration("0:48:52")).toBe(2932);
    expect(parseDuration("48:52")).toBe(2932);
    expect(parseDuration("1:75:00")).toBeNull();
    expect(parseDuration("abc")).toBeNull();
  });

  it("formats total hours", () => {
    expect(formatHours(7200 + 5 * 60)).toBe("2 h 05 min");
  });

  it("rounds paces to whole seconds", () => {
    expect(formatPace(6.0464)).toBe("6:03");
    expect(formatPace(5.9999)).toBe("6:00");
    expect(formatPace(0)).toBe("–");
  });

  it("normalises stored MM:SS values", () => {
    expect(formatMinSec("06:45")).toBe("6:45");
    expect(formatMinSec("08:60")).toBe("9:00");
    expect(formatMinSec("00:00")).toBe("0:00");
    expect(formatMinSec(null)).toBe("–");
  });
});
