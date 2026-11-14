import { describe, expect, it } from "vitest";

import { formatBytes, isBirthday, validateProfile, validateZones } from "./validation";

const valid = {
  name: "Axel",
  weight: "68",
  height: "168",
  birthday: "31.10.1968",
  hrMin: "50",
  hrMax: "185",
};

describe("settings validation", () => {
  it("accepts a complete profile", () => {
    expect(validateProfile(valid)).toEqual({});
  });

  it("flags each invalid field", () => {
    const errors = validateProfile({ ...valid, name: " ", weight: "abc", hrMin: "190" });
    expect(Object.keys(errors).sort()).toEqual(["hrMin", "name", "weight"]);
    expect(validateProfile({ ...valid, hrMin: "120", hrMax: "110" }).hrMax).toBeDefined();
  });

  it("checks birthdays as real past dates", () => {
    expect(isBirthday("31.10.1968")).toBe(true);
    expect(isBirthday("31.02.1990")).toBe(false);
    expect(isBirthday("1968-10-31")).toBe(false);
    expect(isBirthday("01.01.2999")).toBe(false);
  });

  it("requires ascending zones", () => {
    expect(validateZones(["98", "114", "131", "147", "164"])).toBeNull();
    expect(validateZones(["98", "90", "131", "147", "164"])).toMatch(/above/);
    expect(validateZones(["98", "", "131", "147", "164"])).toMatch(/whole number/);
  });

  it("formats sizes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toMatch(/^1[.,]5 KB$/);
    expect(formatBytes(166 * 1024 * 1024)).toBe("166 MB");
  });
});
