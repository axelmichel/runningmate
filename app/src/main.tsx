import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import ReactDOM from "react-dom/client";

import "@/i18n";
import App from "./App";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { startPrivacyMode } from "@/lib/privacy";
import "./index.css";

// Screenshots for the docs: `VITE_PRIVACY=1 pnpm tauri dev` blurs titles, comments and figures.
if (import.meta.env.DEV && import.meta.env.VITE_PRIVACY) {
  startPrivacyMode();
}

// Data comes from the local database, so there is nothing to refetch on focus.
const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: false } },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TooltipProvider>
          <App />
          <Toaster />
        </TooltipProvider>
      </ThemeProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
