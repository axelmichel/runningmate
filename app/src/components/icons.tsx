import { createLucideIcon } from "lucide-react";

/**
 * A walking figure in lucide's style (24 px grid, 2 px round strokes). Lucide itself only has a
 * standing figure (`PersonStanding`).
 */
export const PersonWalking = createLucideIcon("person-walking", [
  ["circle", { cx: "13", cy: "4", r: "1", key: "head" }],
  ["path", { d: "M12.5 7.5 10.5 13.5", key: "torso" }],
  ["path", { d: "m8 11 4.5-3.5 2.5 3.5", key: "arms" }],
  ["path", { d: "m6.5 21 2-3.5 2-4", key: "back-leg" }],
  ["path", { d: "m10.5 13.5 2.5 3.5 1 4", key: "front-leg" }],
]);
