/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import de from "./de.json";
import en from "./en.json";

const SRC = fileURLToPath(new URL("..", import.meta.url));

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name: string) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "bindings" ? [] : sourceFiles(path);
    return /\.(ts|tsx)$/.test(name) && !name.endsWith(".test.ts") ? [path] : [];
  });
}

/** Every literal passed to `t(...)` or `tk(...)`, plus where it was found. */
function usedKeys(): Map<string, string> {
  const keys = new Map<string, string>();
  const call = /\bt(?:k)?\(\s*"((?:[^"\\]|\\.)*)"/g;
  for (const file of sourceFiles(SRC)) {
    for (const match of readFileSync(file, "utf8").matchAll(call)) {
      const key = JSON.parse(`"${match[1]}"`) as string;
      if (key) keys.set(key, file.slice(SRC.length));
    }
  }
  return keys;
}

const german = de as Record<string, string>;
const english = en as Record<string, string>;

describe("translations", () => {
  const keys = usedKeys();

  it("finds the texts in the source", () => {
    expect(keys.size).toBeGreaterThan(200);
  });

  it("has a German text for every key", () => {
    const missing = [...keys]
      .filter(([key]) =>
        key.includes("{{count}}")
          ? !(german[`${key}_one`] && german[`${key}_other`])
          : !german[key],
      )
      .map(([key, file]) => `${key}  (${file})`);
    expect(missing).toEqual([]);
  });

  it("has English plural forms for counted texts", () => {
    const missing = [...keys.keys()].filter(
      (key) => key.includes("{{count}}") && !(english[`${key}_one`] && english[`${key}_other`]),
    );
    expect(missing).toEqual([]);
  });

  it("keeps placeholders in German texts", () => {
    const placeholders = (text: string): string[] => [...(text.match(/\{\{\w+\}\}/g) ?? [])].sort();
    const broken = Object.entries(german).filter(([key, text]) => {
      const base = key.replace(/_(one|other)$/, "");
      const expected = placeholders(base).filter(
        (p) => p !== "{{count}}" || !key.match(/_(one|other)$/),
      );
      return expected.some((p) => !placeholders(text).includes(p));
    });
    expect(broken.map(([key]) => key)).toEqual([]);
  });

  it("has no German entries the code no longer uses", () => {
    const unused = Object.keys(german).filter((key) => !keys.has(key.replace(/_(one|other)$/, "")));
    expect(unused).toEqual([]);
  });
});
