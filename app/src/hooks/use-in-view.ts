import { useEffect, useRef, useState } from "react";

/**
 * Becomes true once the element is near the viewport and stays true, so expensive content
 * (maps, track data) only loads for what is scrolled to.
 */
export function useInView<T extends Element>(margin = "200px") {
  const ref = useRef<T>(null);
  const [inView, setInView] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element || inView) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) setInView(true);
      },
      { rootMargin: margin },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [inView, margin]);
  return [ref, inView] as const;
}
