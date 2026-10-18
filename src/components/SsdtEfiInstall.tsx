import { useEffect, useState } from "react";
import {
  AlertCircle,
  FileDown,
  FolderDown,
  Loader2,
  PackageCheck,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import type { KextDlProgress, SsdtDlItem, SsdtEntry, SsdtsToEfiReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";

interface Props {
  ssdts: SsdtEntry[];
  /** Найденная ESP-база из детектора (undefined — диалог выбора папки) */
  base?: string | null;
}

type Phase = "idle" | "running" | "done";

export function SsdtEfiInstall({ ssdts, base }: Props) {
  const { t, tb } = useI18n();
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<KextDlProgress | null>(null);
  const [report, setReport] = useState<SsdtsToEfiReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<KextDlProgress>("ssdt-dl-progress", (e) => setProgress(e.payload));
    return () => {
      un.then((off) => off());
    };
  }, []);

  // прекомпила DMAR в репозитории Dortania нет — такие таблицы только вручную
  const installable = ssdts.filter((s) => s.selected && s.repo_file);
  const manual = ssdts.filter((s) => s.selected && !s.repo_file);

  const install = async () => {
    setPhase("running");
    setReport(null);
    setError(null);
    setProgress({ done: 0, total: installable.length, kext: installable[0]?.name ?? "" });
    try {
      const items: SsdtDlItem[] = installable.map((s) => ({
        name: s.name,
        file: s.file,
        repo_file: s.repo_file,
      }));
      const r = await invoke<SsdtsToEfiReport>("download_ssdts_to_efi", {
        ssdts: items,
        base: base ?? undefined,
      });
      setReport(r);
      setPhase("done");
    } catch (e) {
      const msg = String(e);
      // пользователь закрыл диалог выбора папки — молча возвращаемся
      if (!msg.includes("отменён") && !msg.includes("отменено")) setError(msg);
      setPhase("idle");
    } finally {
      setProgress(null);
    }
  };

  const okCount = report?.results.filter((r) => r.ok).length ?? 0;
  const failed = report?.results.filter((r) => !r.ok) ?? [];
  const pct = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;

  return (
    <div className="space-y-3 border-t pt-4">
      <div className="flex items-start gap-2">
        <FileDown className="text-muted-foreground mt-0.5 size-4 shrink-0" />
        <p className="text-muted-foreground text-xs leading-relaxed">{t("ssdt.efiDesc")}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button onClick={install} disabled={phase === "running" || installable.length === 0} className="gap-2">
          {phase === "running" ? <Loader2 className="size-4 animate-spin" /> : <FolderDown className="size-4" />}
          {phase === "running" ? t("ssdt.downloading") : t("ssdt.download")}
        </Button>
        {installable.length === 0 && (
          <span className="text-muted-foreground text-xs">{t("ssdt.selectFirst")}</span>
        )}
      </div>

      {manual.length > 0 && (
        <div className="text-muted-foreground text-xs">
          {t("ssdt.manual", { names: manual.map((s) => s.file).join(", ") })}
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
            <div className="min-w-0 text-xs leading-relaxed">
              <div className="font-medium">
                {t("ssdt.done", { ok: okCount, total: report.results.length })}
              </div>
              <div className="text-muted-foreground break-all">{tb(report.acpi_dir)}</div>
              {report.config_path && (
                <div className="mt-1">
                  <div>{t("ssdt.configPatched", { n: report.registered })}</div>
                  {report.backup_made && (
                    <div className="text-muted-foreground break-all">
                      {t("ssdt.configBackup", { path: report.config_path })}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
          {failed.length > 0 && (
            <div className="space-y-1 rounded-md bg-destructive/10 p-3 text-destructive">
              <div className="flex items-center gap-2 text-xs font-medium">
                <AlertCircle className="size-3.5" />
                {t("ssdt.failed", { n: failed.length })}
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
    </div>
  );
}
