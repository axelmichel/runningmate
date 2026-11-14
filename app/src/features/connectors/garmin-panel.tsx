import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, LogOut, Upload } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ImportProgressView } from "@/features/import/import-progress";
import { useImportRun } from "@/features/import/use-import-run";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatDateTime } from "@/lib/format";

import { OPEN_RANGE, toDateRange, type RangeInput } from "./range";
import { RangeFields } from "./range-fields";

/** Error text the backend uses when the stored Garmin token no longer works. */
const SIGN_IN_REQUIRED = "garmin-sign-in-required";

function SignInForm({ onSignedIn }: { onSignedIn: () => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [needsCode, setNeedsCode] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const signIn = useMutation({
    mutationFn: () => api.garminSignIn(username, password),
    onSuccess: (result) => {
      setError(null);
      if (result.status === "needsCode") setNeedsCode(true);
      else onSignedIn();
    },
    onError: (e) => setError(String(e)),
  });
  const submitCode = useMutation({
    mutationFn: () => api.garminSubmitCode(code),
    onSuccess: onSignedIn,
    onError: (e) => setError(String(e)),
  });
  const busy = signIn.isPending || submitCode.isPending;

  return (
    <form
      className="grid max-w-sm gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (needsCode) submitCode.mutate();
        else signIn.mutate();
      }}
    >
      {needsCode ? (
        <div className="grid gap-1.5">
          <Label htmlFor="garmin-code">{t("Verification code")}</Label>
          <Input
            id="garmin-code"
            inputMode="numeric"
            autoComplete="one-time-code"
            autoFocus
            value={code}
            onChange={(e) => setCode(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            {t("Garmin sent a code to your e-mail or authenticator app.")}
          </p>
        </div>
      ) : (
        <>
          <div className="grid gap-1.5">
            <Label htmlFor="garmin-user">{t("E-mail address")}</Label>
            <Input
              id="garmin-user"
              type="email"
              autoComplete="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="garmin-password">{t("Password")}</Label>
            <Input
              id="garmin-password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            {t(
              "Your password is only sent to Garmin. RunningMate keeps a sign-in token in the system keychain, not the password.",
            )}
          </p>
        </>
      )}
      {error && <p className="text-xs break-words text-destructive">{error}</p>}
      <div>
        <Button
          type="submit"
          disabled={busy || (needsCode ? code.trim() === "" : !username.trim() || !password)}
        >
          {busy ? t("Signing in…") : needsCode ? t("Confirm") : t("Sign in")}
        </Button>
      </div>
    </form>
  );
}

/** "YYYY-MM-DD" for a date input, `days` days ago. */
function daysAgo(days: number): string {
  const date = new Date(Date.now() - days * 86_400_000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="grid gap-3 border-t pt-4 first:border-t-0 first:pt-0">
      <h3 className="text-sm font-medium">{title}</h3>
      {children}
    </section>
  );
}

/** Garmin Connect: import new activities and upload ones recorded elsewhere. */
export function GarminPanel({ onRunningChange }: { onRunningChange: (running: boolean) => void }) {
  const queryClient = useQueryClient();
  const importRun = useImportRun();
  const exportRun = useImportRun();
  const [since, setSince] = useState(() => daysAgo(30));
  const [importRange, setImportRange] = useState<RangeInput>(OPEN_RANGE);
  const status = useQuery({ queryKey: ["garmin"], queryFn: api.garminStatus });
  const sinceTimestamp = Math.floor(new Date(`${since}T00:00`).getTime() / 1000);
  const candidates = useQuery({
    queryKey: ["garminCandidates", sinceTimestamp],
    queryFn: () => api.garminExportCandidates(sinceTimestamp),
    enabled: Boolean(status.data?.signedIn) && Number.isFinite(sinceTimestamp),
  });
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: ["garmin"] });
    void queryClient.invalidateQueries({ queryKey: ["garminCandidates"] });
  };
  const signOut = useMutation({ mutationFn: api.garminSignOut, onSettled: refresh });
  const running = importRun.run.running || exportRun.run.running;
  const runError = importRun.run.error ?? exportRun.run.error;
  // A stored token that Garmin no longer accepts sends the user back to the sign-in form.
  const signedIn = (status.data?.signedIn ?? false) && !runError?.includes(SIGN_IN_REQUIRED);

  useEffect(() => onRunningChange(running), [running, onRunningChange]);
  useEffect(() => {
    if (importRun.run.summary || exportRun.run.summary) refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refresh only after a run finishes
  }, [importRun.run.summary, exportRun.run.summary]);

  if (!status.data) return null;
  if (!signedIn) {
    return (
      <SignInForm
        onSignedIn={() => {
          importRun.reset();
          exportRun.reset();
          refresh();
        }}
      />
    );
  }
  const count = candidates.data?.length ?? 0;
  const started = (run: typeof importRun.run) =>
    run.running || run.summary !== null || run.error !== null;

  return (
    <div className="grid gap-4">
      <Section title={t("Account")}>
        <div className="flex items-center justify-between gap-3 text-sm">
          <span className="text-muted-foreground">{t("Signed in to Garmin Connect.")}</span>
          <Button
            variant="ghost"
            size="sm"
            disabled={running || signOut.isPending}
            onClick={() => signOut.mutate()}
          >
            <LogOut />
            {t("Sign out")}
          </Button>
        </div>
      </Section>

      <Section title={t("Import from Garmin Connect")}>
        <p className="text-sm text-muted-foreground">
          {t("Last sync: {{date}}", {
            // Syncs from the Python app only left the synced day behind.
            date: status.data.lastRun
              ? formatDateTime(status.data.lastRun)
              : status.data.lastSync
                ? formatDate(Date.parse(status.data.lastSync) / 1000)
                : t("never"),
          })}
          {status.data.lastSync &&
            ` · ${t("The next sync starts at {{date}}.", {
              date: formatDate(Date.parse(status.data.lastSync) / 1000),
            })}`}
        </p>
        <RangeFields
          id="garmin-import"
          value={importRange}
          onChange={setImportRange}
          disabled={running}
          hint={t(
            "Optional; without dates everything since the last sync. Newest activities are imported first.",
          )}
        />
        {started(importRun.run) && (
          <ImportProgressView run={importRun.run} onCancel={importRun.cancel} />
        )}
        <div>
          <Button
            disabled={running}
            onClick={() => importRun.start(() => api.garminSync(toDateRange(importRange)))}
          >
            <Download />
            {importRun.run.running ? t("Syncing…") : t("Sync now")}
          </Button>
        </div>
      </Section>

      <Section title={t("Upload to Garmin Connect")}>
        <p className="text-sm text-muted-foreground">
          {t(
            "Uploads activities that didn't come from Garmin and aren't on your account yet, e.g. ones recorded with another app. Garmin skips activities it already has.",
          )}
        </p>
        <div className="flex flex-wrap items-end gap-3">
          <div className="grid gap-1.5">
            <Label htmlFor="garmin-since">{t("Activities since")}</Label>
            <Input
              id="garmin-since"
              type="date"
              className="w-44"
              value={since}
              disabled={running}
              onChange={(e) => setSince(e.target.value)}
            />
          </div>
          <Button
            disabled={running || count === 0}
            onClick={() => {
              const ids = candidates.data?.map((c) => c.id) ?? [];
              void exportRun.start(() => api.garminUpload(ids));
            }}
          >
            <Upload />
            {exportRun.run.running
              ? t("Uploading…")
              : count > 0
                ? t("Upload {{count}} activity", { count })
                : t("Nothing to upload")}
          </Button>
        </div>
        {started(exportRun.run) && (
          <ImportProgressView run={exportRun.run} onCancel={exportRun.cancel} />
        )}
      </Section>
    </div>
  );
}
