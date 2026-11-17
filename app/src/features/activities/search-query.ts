import type { ActivityFilter } from "@/bindings/ActivityFilter";

/**
 * What the search box asks for: distance shortcuts become a range in km, the other words are
 * searched in title, comment and place name (each word has to match).
 *
 * - `10km`, `10 km`, `10,5 km`: about that distance (±5 %)
 * - `>21`, `>= 21km`, `≥21`: at least; `<5km`, `<=5`, `≤5`: at most
 * - `10-15km`, `10 – 15 km`, `10km-15km`: between
 *
 * A bare number (`2024`, `5`) stays text. Several shortcuts narrow each other down.
 */
export type SearchQuery = {
  text: string;
  minDistance?: number;
  maxDistance?: number;
};

/** Share of the distance a plain `10km` may be off by. */
const TOLERANCE = 0.05;

const NUMBER = String.raw`\d+(?:[.,]\d+)?`;
const EXACT = new RegExp(String.raw`^(${NUMBER})km$`, "i");
const RANGE = new RegExp(String.raw`^(${NUMBER})(?:km)?[-–](${NUMBER})km$`, "i");
const COMPARE = new RegExp(String.raw`^(>=|<=|>|<|≥|≤)(${NUMBER})(?:km)?$`, "i");

const toNumber = (value: string) => Number(value.replace(",", "."));
const round = (value: number) => Math.round(value * 1000) / 1000;

export function parseSearch(input: string): SearchQuery {
  // Join "10 km", "> 21" and "10 - 15" into single tokens.
  const normalized = input
    .replace(/(\d)\s+(km)\b/gi, "$1$2")
    .replace(/(>=|<=|>|<|≥|≤)\s+(?=\d)/g, "$1")
    .replace(new RegExp(String.raw`(\d(?:km)?)\s*([-–])\s*(?=${NUMBER}km\b)`, "gi"), "$1$2");
  let min: number | undefined;
  let max: number | undefined;
  const atLeast = (value: number) => (min = min == null ? value : Math.max(min, value));
  const atMost = (value: number) => (max = max == null ? value : Math.min(max, value));
  const words: string[] = [];
  for (const token of normalized.split(/\s+/).filter(Boolean)) {
    const exact = EXACT.exec(token);
    const range = RANGE.exec(token);
    const compare = COMPARE.exec(token);
    if (exact) {
      const value = toNumber(exact[1]);
      atLeast(round(value * (1 - TOLERANCE)));
      atMost(round(value * (1 + TOLERANCE)));
    } else if (range) {
      const [a, b] = [toNumber(range[1]), toNumber(range[2])];
      atLeast(Math.min(a, b));
      atMost(Math.max(a, b));
    } else if (compare) {
      const value = toNumber(compare[2]);
      if (/[>≥]/.test(compare[1])) atLeast(value);
      else atMost(value);
    } else {
      words.push(token);
    }
  }
  return { text: words.join(" "), minDistance: min, maxDistance: max };
}

/**
 * The filter sent to the backend: the search box's distance shortcuts intersect the filter
 * dialog's distance range (both have to hold), the rest of the box is the text search.
 */
export function withSearch(filter: ActivityFilter): ActivityFilter {
  if (!filter.searchText) return filter;
  const query = parseSearch(filter.searchText);
  const pick = (a: number | undefined, b: number | undefined, choose: typeof Math.max) =>
    a == null ? b : b == null ? a : choose(a, b);
  const result: ActivityFilter = { ...filter, searchText: query.text };
  if (!query.text) delete result.searchText;
  const minDistance = pick(filter.minDistance, query.minDistance, Math.max);
  const maxDistance = pick(filter.maxDistance, query.maxDistance, Math.min);
  if (minDistance != null) result.minDistance = minDistance;
  if (maxDistance != null) result.maxDistance = maxDistance;
  return result;
}
