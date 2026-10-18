import { useEffect, useState } from "react";
import {
  AlertCircle,
  FolderDown,
  HardDriveDownload,
  Loader2,
  PackageCheck,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import type { FullEfiProgress, FullEfiReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

type Phase = "idle" | "running" | "done";

export function FullEfiBuildCard() {
  const { t, tb } = useI18n();
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<FullEfiProgress | null>(null);
  const [report, setReport] = useState<FullEfiReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<FullEfiProgress>("fullefi-progress", (e) => setProgress(e.payload));
    return () => {
      un.then((off) => off());
    };
  }, []);

  const build = async () => {
    setPhase("running");
    setReport(null);
    setError(null);
    setProgress({ stage: "prepare", message: "", percent: 0 });
    try {
      const r = await invoke<FullEfiReport>("build_full_efi_to_efi");
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

  return (
    <div className="space-y-3">
      <div className="flex items-start gap-2">
        <HardDriveDownload className="text-muted-foreground mt-0.5 size-4 shrink-0" />
        <p className="text-muted-foreground text-xs leading-relaxed">{t("fullefi.desc")}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button onClick={build} disabled={phase === "running"} className="gap-2">
          {phase === "running" ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <FolderDown className="size-4" />
          )}
          {phase === "running" ? t("fullefi.building") : t("fullefi.build")}
        </Button>
        <span className="text-muted-foreground text-xs">{t("fullefi.includes")}</span>
      </div>

      {phase === "running" && progress && (
        <div className="space-y-1.5">
          <div className="text-muted-foreground flex items-center gap-2 text-xs">
            <Loader2 className="size-3.5 animate-spin" />
            {progress.message ? tb(progress.message) : t("fullefi.preparing")}
            <span className="ml-auto tabular-nums">{progress.percent}%</span>
          </div>
          <div className="bg-muted h-1.5 overflow-hidden rounded-full">
            <div
              className="bg-primary h-full rounded-full transition-all"
              style={{ width: `${progress.percent}%` }}
            />
          </div>
        </div>
      )}

      {phase === "done" && report && (
        <div className="space-y-2">
          <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
            <PackageCheck className="mt-0.5 size-4 shrink-0" />
            <div className="min-w-0 space-y-1 text-xs leading-relaxed">
              <div className="font-medium">
                {t("fullefi.done", { version: report.opencore_version ?? "?" })}
              </div>
              <div className="text-muted-foreground break-all">{tb(report.efi_dir)}</div>
              <div className="flex flex-wrap gap-1 pt-1">
                {report.files_installed.map((f) => (
                  <Badge key={f} variant="secondary" className="font-mono text-[10px]">
                    {f}
                  </Badge>
                ))}
              </div>
              <div>
                {t("fullefi.bootx64Label")} {t(`fullefi.bootx64.${report.bootx64}`)}
              </div>
              {report.bootx64_note && (
                <div className="text-amber-600 dark:text-amber-400">{tb(report.bootx64_note)}</div>
              )}
              <div>
                {t("fullefi.configLabel")}{" "}
                {report.config_source === "sample"
                  ? t("fullefi.configSample")
                  : t("fullefi.configExisting")}
                {report.config_patched && ` · ${t("fullefi.configPatched")}`}
                {report.canopy_enabled && ` · ${t("fullefi.canopy")}`}
                {report.resources_installed.length > 0 &&
                  ` · ${t("fullefi.resources", { dirs: report.resources_installed.join(", ") })}`}
              </div>
              {report.config_backup && (
                <div className="text-muted-foreground break-all">
                  {t("fullefi.configBackup", { path: report.config_path ?? "" })}
                </div>
              )}
              {report.backup_path && (
                <div className="text-muted-foreground break-all">
                  {t("fullefi.backup", { path: report.backup_path })}
                </div>
              )}
            </div>
          </div>

          {(report.warnings.length > 0 || (report.audit_warnings?.length ?? 0) > 0) && (
            <div className="space-y-1 rounded-md bg-amber-500/10 p-3 text-amber-600 dark:text-amber-400">
              <div className="flex items-center gap-2 text-xs font-medium">
                <AlertCircle className="size-3.5" />
                {t("fullefi.warnings")}
              </div>
              <ul className="space-y-0.5 text-xs leading-relaxed">
                {report.warnings.map((w, i) => (
                  <li key={`w${i}`}>{tb(w)}</li>
                ))}
                {(report.audit_warnings ?? []).map((w, i) => (
                  <li key={`a${i}`}>{tb(w)}</li>
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
