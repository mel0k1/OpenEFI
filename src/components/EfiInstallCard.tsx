import { useEffect, useState } from "react";
import {
  AlertCircle,
  FolderDown,
  Loader2,
  PackageCheck,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import type { KextDlProgress, KextEntry, KextsToEfiReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  kexts: KextEntry[];
  darwin: number;
  /** Найденная ESP-база из детектора (undefined — диалог выбора папки) */
  base?: string | null;
}

type Phase = "idle" | "running" | "done";

export function EfiInstallCard({ kexts, darwin, base }: Props) {
  const { t, tb } = useI18n();
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<KextDlProgress | null>(null);
  const [report, setReport] = useState<KextsToEfiReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<KextDlProgress>("kext-dl-progress", (e) => setProgress(e.payload));
    return () => {
      un.then((off) => off());
    };
  }, []);

  // в EFI скачиваются только EFI-кексты; UTBMap — через помощник USB, драйверы — в macOS
  const installable = kexts.filter((k) => k.selected && !k.non_efi && k.name !== "UTBMap");
  const excluded = kexts.filter((k) => k.selected && (k.non_efi || k.name === "UTBMap"));

  const install = async () => {
    setPhase("running");
    setReport(null);
    setError(null);
    setProgress({ done: 0, total: installable.length, kext: installable[0]?.name ?? "" });
    try {
      const r = await invoke<KextsToEfiReport>("download_kexts_to_efi", {
        kexts: installable.map((k) => ({ name: k.name, repo: k.repo })),
        darwin,
        base: base ?? undefined,
      });
      setReport(r);
      setPhase("done");
    } catch (e) {
      const msg = String(e);
      // пользователь закрыл диалог выбора папки — молча возвращаемся
      if (!msg.includes("отменён")) setError(msg);
      setPhase("idle");
    } finally {
      setProgress(null);
    }
  };

  const okCount = report?.results.filter((r) => r.ok).length ?? 0;
  const failed = report?.results.filter((r) => !r.ok) ?? [];
  const pct = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">{t("efi.title")}</CardTitle>
        <CardDescription className="text-xs">{t("efi.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={install} disabled={phase === "running" || installable.length === 0} className="gap-2">
            {phase === "running" ? <Loader2 className="size-4 animate-spin" /> : <FolderDown className="size-4" />}
            {phase === "running" ? t("efi.downloading") : t("efi.download")}
          </Button>
          {installable.length === 0 && (
            <span className="text-muted-foreground text-xs">{t("efi.selectFirst")}</span>
          )}
        </div>

        {excluded.length > 0 && (
          <div className="text-muted-foreground text-xs">
            {t("efi.excluded", { names: excluded.map((k) => k.name).join(", ") })}
          </div>
        )}

        {phase === "running" && progress && (
          <div className="space-y-1.5">
            <div className="text-muted-foreground flex items-center gap-2 text-xs">
              <Loader2 className="size-3.5 animate-spin" />
              {progress.kext} — {t("efi.progress", { done: progress.done, total: progress.total })}
            </div>
            <div className="bg-muted h-1.5 overflow-hidden rounded-full">
              <div className="bg-primary h-full rounded-full transition-all" style={{ width: `${pct}%` }} />
            </div>
          </div>
        )}

        {phase === "done" && report && (
          <div className="space-y-2">
            <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
              <PackageCheck className="mt-0.5 size-4 shrink-0" />
              <div className="text-xs leading-relaxed">
                <div className="font-medium">
                  {t("efi.done", { ok: okCount, total: report.results.length })}
                </div>
                <div className="text-muted-foreground break-all">{tb(report.kexts_dir)}</div>
              </div>
            </div>
            {failed.length > 0 && (
              <div className="space-y-1 rounded-md bg-destructive/10 p-3 text-destructive">
                <div className="flex items-center gap-2 text-xs font-medium">
                  <AlertCircle className="size-3.5" />
                  {t("efi.failed", { n: failed.length })}
                </div>
                <ul className="space-y-0.5 text-xs leading-relaxed">
                  {failed.map((f) => (
                    <li key={f.name}>
                      <span className="font-medium">{f.name}</span>
                      {f.message ? ` — ${tb(f.message)}` : ""}
                    </li>
                  ))}
                </ul>
              </div>
            )}
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
