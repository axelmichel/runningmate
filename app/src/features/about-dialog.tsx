import { useQuery } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";

import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { t } from "@/i18n";
import { api } from "@/lib/api";

const DOCS_URL = "https://axelmichel.github.io/runningmate/";
const REPO_URL = "https://github.com/axelmichel/runningmate";

/** About window with the RunningMate logo, version and links. */
export function AboutDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { data: info } = useQuery({ queryKey: ["appInfo"], queryFn: api.appInfo });
  const link = (url: string, label: string) => (
    <button
      type="button"
      className="text-left text-[#e13342] hover:underline"
      onClick={() => void openUrl(url)}
    >
      {label}
    </button>
  );
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="grid max-w-2xl grid-cols-[1fr_auto] gap-0 overflow-hidden bg-[#111] p-0 text-white sm:max-w-2xl">
        <div className="grid content-between gap-6 p-8">
          <div className="grid gap-4">
            <img src="/logo.svg" alt="" className="w-40" />
            <div>
              <DialogTitle className="text-3xl font-semibold text-[#e13342]">
                RunningMate
              </DialogTitle>
              <p className="text-right text-lg text-white/80">V {info?.version ?? ""}</p>
            </div>
            <DialogDescription className="text-sm leading-relaxed text-white/90">
              {t(
                "An application to monitor your runs, rides and walks. Have an overview of your activities and full control over your data. Forever, for you, for free.",
              )}
            </DialogDescription>
          </div>
          <div className="grid gap-1 text-sm">
            {link(DOCS_URL, "axelmichel.github.io/runningmate")}
            {link(REPO_URL, "github.com/axelmichel/runningmate")}
            <span className="text-xs text-white/60">{t("MIT licence")} © 2025</span>
          </div>
        </div>
        <img src="/about-map.jpg" alt="" className="h-full max-h-[26rem] object-cover" />
      </DialogContent>
    </Dialog>
  );
}
