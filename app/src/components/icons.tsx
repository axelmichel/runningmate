import { createLucideIcon } from "lucide-react";

/**
 * Activity figures in lucide's style (24 px grid, 2 px round strokes, a dot for the head).
 * Lucide itself only has a standing figure (`PersonStanding`).
 */
export const PersonWalking = createLucideIcon("person-walking", [
  ["circle", { cx: "13", cy: "4", r: "1", key: "head" }],
  ["path", { d: "M12.5 7.5 10.5 13.5", key: "torso" }],
  ["path", { d: "m8 11 4.5-3.5 2.5 3.5", key: "arms" }],
  ["path", { d: "m6.5 21 2-3.5 2-4", key: "back-leg" }],
  ["path", { d: "m10.5 13.5 2.5 3.5 1 4", key: "front-leg" }],
]);

/** A running figure matching `PersonWalking` (lucide has no runner either). */
export const PersonRunning = createLucideIcon("person-running", [
  ["circle", { cx: "15", cy: "4", r: "1", key: "head" }],
  ["path", { d: "M14 7.5 11 13", key: "torso" }],
  ["path", { d: "m8.5 10.5 2-3H14l2.5 3H19", key: "arms" }],
  ["path", { d: "M11 13l3.5 2.5-.5 5", key: "front-leg" }],
  ["path", { d: "m11 13-2.5 4H4.5", key: "back-leg" }],
]);
