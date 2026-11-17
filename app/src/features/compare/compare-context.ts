import { createContext, useContext } from "react";

import type { CompareSelection } from "./compare-math";

/**
 * Opens the Compare page with a new selection, from anywhere in the app (the segment window
 * is opened from several pages and from within the detail window). Provided by `App`.
 */
export const OpenCompareContext = createContext<((selection: CompareSelection) => void) | null>(
  null,
);

export function useOpenCompare() {
  return useContext(OpenCompareContext);
}
