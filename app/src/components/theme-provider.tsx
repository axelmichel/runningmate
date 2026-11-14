import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { ThemeContext, type Theme, type ThemeContextValue } from "@/hooks/use-theme";

const STORAGE_KEY = "runningmate-theme";

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem(STORAGE_KEY) as Theme | null) ?? "system",
  );

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || (theme === "system" && media.matches);
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  // Stable identity, so effects can depend on it.
  const changeTheme = useCallback((next: Theme) => {
    localStorage.setItem(STORAGE_KEY, next);
    setTheme(next);
  }, []);
  const value = useMemo<ThemeContextValue>(
    () => ({ theme, setTheme: changeTheme }),
    [theme, changeTheme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}
