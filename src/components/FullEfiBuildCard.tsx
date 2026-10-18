import { useEffect, useState } from "react";
import {
  AlertCircle,
  FolderDown,
  HardDriveDownload,
  Loader2,
  PackageCheck,
  Search,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import type { FullEfiProgress, FullEfiReport, OpenCoreStatus } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";

type Phase = "idle" | "running" | "done";

interface Props {
  /** Найденная ESP-база из детектора (undefined — диалог выбора папки) */
  base?: string | null;
}

export function FullEfiBuildCard({ base }: Props) {
  const { t, tb } = useI18n();
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<FullEfiProgress | null>(null);
  const [report, setReport] = useState<FullEfiReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tools, setTools] = useState(false);
  const [audioDxe, setAudioDxe] = useState(false);
  const [status, setStatus] = useState<OpenCoreStatus | null>(null);
  const [statusChecking, setStatusChecking] = useState(false);

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
      const r = await invoke<FullEfiReport>("build_full_efi_to_efi", {
        base: base ?? undefined,
        tools,
        audioDxe,
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

  const checkStatus = async () => {
    setStatusChecking(true);
    setError(null);
    setStatus(null);
    try {
      setStatus(await invoke<OpenCoreStatus>("opencore_status", { base: base ?? undefined }));
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменён")) setError(msg);
    } finally {
      setStatusChecking(false);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-start gap-2">
        <HardDriveDownload className="text-muted-foreground mt-0.5 size-4 shrink-0" />
        <p className="text-muted-foreground text-xs leading-relaxed">{t("fullefi.desc")}</p>
      </div>

      {/* Состояние установленного OpenCore */}
      <div className="space-y-2 rounded-lg border p-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" size="sm" onClick={checkStatus} disabled={statusChecking} className="gap-2">
            {statusChecking ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <Search className="size-3.5" />
            )}
            {statusChecking ? t("fullefi.statusChecking") : t("fullefi.statusCheck")}
          </Button>
          <span className="text-muted-foreground text-xs">{t("fullefi.statusTitle")}</span>
        </div>
        {status && (
          <div className="text-xs leading-relaxed">
            {!status.installed ? (
              <span className="text-muted-foreground">{t("fullefi.notInstalled")}</span>
            ) : status.version && status.latest ? (
              <span className={status.version === status.latest ? "text-[var(--success)]" : "text-amber-600 dark:text-amber-400"}>
                {t("fullefi.installedVersion", { v: status.version, latest: status.latest })}
              </span>
            ) : (
              <span className="text-muted-foreground">{t("fullefi.installedUnknown")}</span>
            )}
          </div>
        )}
      </div>

      {/* Опциональные компоненты */}
      <div className="space-y-2 rounded-lg border p-3">
        <div className="text-xs font-medium">{t("fullefi.options")}</div>
        <div className="flex items-start gap-2">
          <Checkbox id="fullefi-tools" checked={tools} onCheckedChange={(v) => setTools(v === true)} />
          <Label htmlFor="fullefi-tools" className="text-xs leading-relaxed font-normal">
            {t("fullefi.tools")}
          </Label>
        </div>
        <div className="flex items-start gap-2">
          <Checkbox id="fullefi-audio" checked={audioDxe} onCheckedChange={(v) => setAudioDxe(v === true)} />
          <Label htmlFor="fullefi-audio" className="text-xs leading-relaxed font-normal">
            {t("fullefi.audio")}
          </Label>
        </div>
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
                {report.was_up_to_date === true
                  ? t("fullefi.wasUpToDate", { v: report.opencore_version ?? "?" })
                  : report.previous_version && report.previous_version !== report.opencore_version
                    ? t("fullefi.updatedFrom", {
                        old: report.previous_version,
                        new: report.opencore_version ?? "?",
                      })
                    : t("fullefi.done", { version: report.opencore_version ?? "?" })}
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
              {(report.tools_installed?.length ?? 0) > 0 && (
                <div>
                  {t("fullefi.toolsInstalled", { n: report.tools_installed!.join(", ") })}
                </div>
              )}
              {report.audio_installed && <div>{t("fullefi.audioInstalled")}</div>}
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
