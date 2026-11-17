/**
 * Privacy mode for screenshots (`VITE_PRIVACY=1`, development only): besides the elements
 * marked `data-private`, every short text with a figure in it, a number with a unit (km, bpm,
 * %, …) or a time like 5:41, gets the class `privacy-figure`, which blurs it (or, in charts,
 * hides it). Only a class is added, so React's DOM stays as it is.
 */
const FIGURE =
  /\d[\d.,]*\s*(?:km\/h|\/km|km|m\b|bpm|spm|kcal|W\b|%|h\b|min\b|s\b|ml\/kg|°C|mm\b|MB|KB|GB)|\d+:\d\d/;
/** Longer texts are sentences; blurring them whole would hide too much. */
const MAX_LENGTH = 80;

function mark(node: Node) {
  const parent = node.parentElement;
  const text = node.textContent ?? "";
  if (!parent || text.length > MAX_LENGTH || parent.closest("[data-private]")) return;
  // Adding a class that is there already still counts as a change, and would loop.
  if (FIGURE.test(text) && !parent.classList.contains("privacy-figure")) {
    parent.classList.add("privacy-figure");
  }
}

function scan(root: Node) {
  if (root.nodeType === Node.TEXT_NODE) return mark(root);
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) mark(node);
}

export function startPrivacyMode() {
  document.documentElement.classList.add("privacy");
  scan(document.body);
  new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === "characterData") mark(mutation.target);
      else if (mutation.type === "attributes") {
        // Only the element's own text: its class was reset, the rest is unchanged.
        mutation.target.childNodes.forEach((child) => {
          if (child.nodeType === Node.TEXT_NODE) mark(child);
        });
      } else mutation.addedNodes.forEach(scan);
    }
  }).observe(document.body, {
    subtree: true,
    childList: true,
    characterData: true,
    // React resets `class` on updates; the scan puts the mark back.
    attributes: true,
    attributeFilter: ["class"],
  });
}
