import { useCallback, useEffect, useState } from "react";
import {
  AlertCircle,
  ArrowUpCircle,
  CheckCircle2,
  Download,
  ExternalLink,
  FolderGit2,
  Languages,
  RefreshCw,
  Settings,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n, dateLocale, type Lang } from "@/i18n";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

const REPO = "mel0k1/OpenEFI";
const RELEASES_API = `https://api.github.com/repos/${REPO}/releases/latest`;

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest" }
  | { kind: "update"; version: string; url: string; notes: string | null; date: string | null }
  | { kind: "error"; message: string };

function parseVersion(v: string): number[] {
  return v
    .trim()
    .replace(/^v/i, "")
    .split(/[-+]/)[0]
    .split(".")
    .map((n) => parseInt(n, 10) || 0);
}

function isNewerVersion(remote: string, current: string): boolean {
  const r = parseVersion(remote);
  const c = parseVersion(current);
  const len = Math.max(r.length, c.length);
  for (let i = 0; i < len; i++) {
    const a = r[i] ?? 0;
    const b = c[i] ?? 0;
    if (a !== b) return a > b;
  }
  return false;
}

async function getAppVersion(): Promise<string> {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return await getVersion();
  } catch {
    // dev-режим в браузере без Tauri-рантайма
    return "0.7.0";
  }
}

interface ReleaseInfo {
  tag_name: string;
  html_url: string;
  body: string | null;
  published_at: string | null;
}

function LanguageSwitch() {
  const { lang, setLang } = useI18n();
  const options: { value: Lang; label: string }[] = [
    { value: "ru", label: "Русский" },
    { value: "en", label: "English" },
  ];
  return (
    <div className="flex gap-1 rounded-lg border p-1">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => setLang(o.value)}
          className={cn(
            "rounded-md px-3 py-1 text-xs font-medium transition-colors",
            lang === o.value
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:bg-accent"
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function SettingsDialog({ children }: { children: React.ReactNode }) {
  const { t, lang } = useI18n();
  const [open, setOpen] = useState(false);
  const [appVersion, setAppVersion] = useState<string>("");
  const [state, setState] = useState<UpdateState>({ kind: "idle" });

  useEffect(() => {
    if (open) {
      setState({ kind: "idle" });
      getAppVersion().then(setAppVersion);
    }
  }, [open]);

  const checkUpdates = useCallback(
    async (currentVersion: string) => {
      setState({ kind: "checking" });
      try {
        const res = await fetch(RELEASES_API, {
          headers: { Accept: "application/vnd.github+json" },
        });
        if (res.status === 404) {
          // релизов ещё нет — значит установленная версия самая свежая
          setState({ kind: "latest" });
          return;
        }
        if (!res.ok) {
          throw new Error(`GitHub API ${res.status}`);
        }
        const rel = (await res.json()) as ReleaseInfo;
        const remote = rel.tag_name;
        if (!remote) {
          throw new Error("GitHub API: invalid response");
        }
        if (isNewerVersion(remote, currentVersion)) {
          setState({
            kind: "update",
            version: remote,
            url: rel.html_url,
            notes: rel.body,
            date: rel.published_at,
          });
        } else {
          setState({ kind: "latest" });
        }
      } catch (e) {
        setState({
          kind: "error",
          message: e instanceof Error ? e.message : t("set.githubError"),
        });
      }
    },
    [t]
  );

  const formatDate = (iso: string | null) => {
    if (!iso) return null;
    try {
      return new Date(iso).toLocaleDateString(dateLocale(lang), {
        day: "numeric",
        month: "long",
        year: "numeric",
      });
    } catch {
      return null;
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings className="size-4.5" />
            {t("set.title")}
          </DialogTitle>
          <DialogDescription>OpenEFI {appVersion ? `v${appVersion}` : ""}</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          {/* Язык / Language */}
          <div className="rounded-lg border p-4">
            <div className="mb-3 flex items-center justify-between gap-2">
              <div>
                <div className="flex items-center gap-1.5 text-sm font-medium">
                  <Languages className="size-3.5" />
                  {t("set.language")}
                </div>
                <div className="text-muted-foreground text-xs">{t("set.langDesc")}</div>
              </div>
            </div>
            <LanguageSwitch />
          </div>

          {/* Обновления */}
          <div className="rounded-lg border p-4">
            <div className="mb-3 flex items-center justify-between gap-2">
              <div>
                <div className="text-sm font-medium">{t("set.updates")}</div>
                <div className="text-muted-foreground text-xs">{t("set.updatesDesc")}</div>
              </div>
              <Button
                size="sm"
                disabled={state.kind === "checking" || !appVersion}
                onClick={() => appVersion && checkUpdates(appVersion)}
              >
                {state.kind === "checking" ? (
                  <>
                    <RefreshCw className="size-3.5 animate-spin" />
                    {t("set.checking")}
                  </>
                ) : (
                  <>
                    <RefreshCw className="size-3.5" />
                    {t("set.check")}
                  </>
                )}
              </Button>
            </div>

            {state.kind === "latest" && (
              <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
                <div className="text-xs leading-relaxed">
                  <div className="font-medium">{t("set.latest")}</div>
                  <div className="text-muted-foreground">
                    {t("set.latestDesc", { v: appVersion })}
                  </div>
                </div>
              </div>
            )}

            {state.kind === "update" && (
              <div className="rounded-md bg-blue-500/10 p-3">
                <div className="flex items-start gap-2 text-blue-600 dark:text-blue-400">
                  <ArrowUpCircle className="mt-0.5 size-4 shrink-0" />
                  <div className="flex-1 text-xs leading-relaxed">
                    <div className="font-medium">
                      {t("set.updateAvailable", { v: state.version })}
                    </div>
                    {state.date && (
                      <div className="text-muted-foreground">
                        {t("set.published", { date: formatDate(state.date) ?? "" })}
                      </div>
                    )}
                  </div>
                </div>
                {state.notes && (
                  <>
                    <p className="text-muted-foreground mt-2 line-clamp-3 text-xs whitespace-pre-line">
                      {state.notes}
                    </p>
                  </>
                )}
                <Button
                  size="sm"
                  className="mt-3 w-full"
                  onClick={() => openUrl(state.url)}
                >
                  <Download className="size-3.5" />
                  {t("set.openRelease")}
                </Button>
              </div>
            )}

            {state.kind === "error" && (
              <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-destructive">
                <AlertCircle className="mt-0.5 size-4 shrink-0" />
                <div className="text-xs leading-relaxed">
                  <div className="font-medium">{t("set.checkFailed")}</div>
                  <div className="text-muted-foreground">{state.message}</div>
                </div>
              </div>
            )}
          </div>

          {/* Ссылки */}
          <div>
            <div className="text-sm font-medium">{t("set.links")}</div>
            <div className="mt-2 grid grid-cols-2 gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => openUrl(`https://github.com/${REPO}`)}
              >
                <FolderGit2 className="size-3.5" />
                {t("set.repo")}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => openUrl(`https://github.com/${REPO}/issues`)}
              >
                <ExternalLink className="size-3.5" />
                {t("set.bug")}
              </Button>
            </div>
          </div>

        </div>
      </DialogContent>
    </Dialog>
  );
}
