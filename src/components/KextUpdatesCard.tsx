import { useEffect, useState } from "react";
import {
  AlertCircle,
  CircleCheck,
  Loader2,
  PackageCheck,
  RefreshCw,
  ArrowUpCircle,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import type { KextDlProgress, KextUpdatesReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  /** Найденная ESP-база из детектора (undefined — диалог выбора папки) */
  base?: string | null;
  darwin: number;
}

type Phase = "idle" | "updating" | "done";

/** Обновление установленных кекстов: версии из Info.plist vs GitHub */
export function KextUpdatesCard({ base, darwin }: Props) {
  const { t, tb } = useI18n();
  const [report, setReport] = useState<KextUpdatesReport | null>(null);
  const [checking, setChecking] = useState(false);
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<KextDlProgress | null>(null);
  const [updatedCount, setUpdatedCount] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<KextDlProgress>("kext-dl-progress", (e) => setProgress(e.payload));
    return () => {
      un.then((off) => off());
    };
  }, []);

  const check = async () => {
    setChecking(true);
    setError(null);
    setReport(null);
    setUpdatedCount(null);
    try {
      setReport(await invoke<KextUpdatesReport>("check_kext_updates", { base: base ?? undefined }));
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменён")) setError(msg);
    } finally {
      setChecking(false);
    }
  };

  const outdated = report?.checks.filter((c) => c.outdated) ?? [];

  const update = async () => {
    setPhase("updating");
    setError(null);
    setProgress({ done: 0, total: outdated.length, kext: outdated[0]?.bundle ?? "" });
    try {
      const r = await invoke<{ results: { ok: boolean }[] }>("update_outdated_kexts", {
        base: base ?? undefined,
        bundles: outdated.map((c) => c.bundle),
        darwin,
      });
      setUpdatedCount(r.results.filter((x) => x.ok).length);
      setPhase("done");
      // перепроверяем версии сразу после обновления
      try {
        setReport(
          await invoke<KextUpdatesReport>("check_kext_updates", { base: base ?? undefined }),
        );
      } catch {
        // повторная проверка не критична
      }
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменён")) setError(msg);
      setPhase("idle");
    } finally {
      setProgress(null);
    }
  };

  const statusLine = (c: KextUpdatesReport["checks"][number]) => {
    if (c.outdated) return t("kupd.outdated", { cur: c.installed ?? "?", latest: c.latest ?? "?" });
    if (c.installed) return t("kupd.latest", { v: c.installed });
    return t("kupd.noVer");
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <ArrowUpCircle className="size-4" />
          {t("kupd.title")}
        </CardTitle>
        <CardDescription className="text-xs">{t("kupd.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" size="sm" onClick={check} disabled={checking || phase === "updating"} className="gap-2">
            {checking ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <RefreshCw className="size-3.5" />
            )}
            {checking ? t("kupd.checking") : t("kupd.check")}
          </Button>
          {report && outdated.length > 0 && (
            <Button size="sm" onClick={update} disabled={phase === "updating"} className="gap-2">
              {phase === "updating" ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <ArrowUpCircle className="size-3.5" />
              )}
              {phase === "updating" ? t("kupd.updating") : t("kupd.updateAll", { n: outdated.length })}
            </Button>
          )}
        </div>

        {phase === "updating" && progress && (
          <div className="space-y-1.5">
            <div className="text-muted-foreground flex items-center gap-2 text-xs">
              <Loader2 className="size-3.5 animate-spin" />
              {progress.kext}
              <span className="ml-auto tabular-nums">
                {progress.done}/{progress.total}
              </span>
            </div>
            <div className="bg-muted h-1.5 overflow-hidden rounded-full">
              <div
                className="bg-primary h-full rounded-full transition-all"
                style={{ width: `${progress.total ? (progress.done / progress.total) * 100 : 0}%` }}
              />
            </div>
          </div>
        )}

        {phase === "done" && updatedCount !== null && (
          <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
            <PackageCheck className="mt-0.5 size-4 shrink-0" />
            <div className="text-xs leading-relaxed">{t("kupd.updated", { n: updatedCount })}</div>
          </div>
        )}

        {report && (
          <div className="space-y-1.5">
            {outdated.length === 0 && phase !== "done" && (
              <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
                <CircleCheck className="mt-0.5 size-4 shrink-0" />
                <div className="text-xs leading-relaxed">{t("kupd.upToDate")}</div>
              </div>
            )}
            {report.checks.map((c) => (
              <div
                key={c.bundle}
                className="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2 text-xs"
              >
                <span className="flex items-center gap-2">
                  <span className="font-mono">{c.bundle}</span>
                  {c.outdated && (
                    <Badge variant="destructive" className="text-[10px]">
                      {c.installed ?? "?"} → {c.latest ?? "?"}
                    </Badge>
                  )}
                </span>
                <span className="text-muted-foreground">
                  {c.message ? tb(c.message) : statusLine(c)}
                </span>
              </div>
            ))}
          </div>
        )}

        {error && (
          <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-destructive">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <div className="text-xs leading-relaxed">{tb(error)}</div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
