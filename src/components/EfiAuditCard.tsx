import { useState } from "react";
import {
  Apple,
  ArchiveRestore,
  CircleCheck,
  CircleX,
  FolderSearch,
  HardDrive,
  Loader2,
  Monitor,
  ShieldCheck,
  TriangleAlert,
  Terminal,
} from "lucide-react";
import type { EfiAudit, EfiBackupReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

/** Карточка защиты EFI: аудит папки + ручной бэкап EFI/OC (шаг Экспорт) */
export function EfiAuditCard() {
  const { t, tb } = useI18n();
  const [audit, setAudit] = useState<EfiAudit | null>(null);
  const [checking, setChecking] = useState(false);
  const [backup, setBackup] = useState<EfiBackupReport | null>(null);
  const [backingUp, setBackingUp] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runAudit = async () => {
    setChecking(true);
    setError(null);
    setBackup(null);
    try {
      setAudit(await invoke<EfiAudit>("audit_efi_folder"));
    } catch (e) {
      setError(String(e));
    } finally {
      setChecking(false);
    }
  };

  const runBackup = async () => {
    setBackingUp(true);
    setError(null);
    try {
      setBackup(await invoke<EfiBackupReport>("backup_efi_folder"));
    } catch (e) {
      setError(String(e));
    } finally {
      setBackingUp(false);
    }
  };

  const kindIcon = (kind: string) => {
    const cls = "size-3.5 shrink-0";
    switch (kind) {
      case "opencore":
        return <ShieldCheck className={`${cls} text-primary`} />;
      case "windows":
        return <Monitor className={`${cls} text-sky-500`} />;
      case "linux":
        return <Terminal className={`${cls} text-amber-500`} />;
      case "clover":
        return <Apple className={`${cls} text-emerald-500`} />;
      default:
        return <FolderSearch className={`${cls} text-muted-foreground`} />;
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <ShieldCheck className="size-4" />
          {t("efia.title")}
        </CardTitle>
        <CardDescription className="text-xs">{t("efia.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 text-xs">
        <ul className="text-muted-foreground space-y-1.5 leading-relaxed">
          {[1, 2, 3, 4].map((i) => (
            <li key={i} className="flex items-start gap-1.5">
              <CircleCheck className="mt-0.5 size-3.5 shrink-0 text-[var(--success)]" />
              {t(`efia.safe.${i}`)}
            </li>
          ))}
        </ul>

        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={runAudit} disabled={checking} className="gap-2">
            {checking ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <FolderSearch className="size-3.5" />
            )}
            {checking ? t("efia.checking") : t("efia.audit")}
          </Button>
          <Button variant="outline" size="sm" onClick={runBackup} disabled={backingUp} className="gap-2">
            {backingUp ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <ArchiveRestore className="size-3.5" />
            )}
            {backingUp ? t("efia.backingUp") : t("efia.backup")}
          </Button>
        </div>

        {backup && (
          <div className="text-[var(--success)] leading-relaxed">
            {t("efia.backupDone", { path: backup.backup_path, n: backup.files })}
          </div>
        )}
        {error && (
          <div className="text-[var(--destructive)] leading-relaxed">
            {t("efia.backupErr", { error: tb(error) })}
          </div>
        )}

        {audit && (
          <div className="space-y-3 rounded-lg border p-3">
            <div className="flex items-center justify-between gap-2">
              <span className="flex items-center gap-1.5 font-medium">
                <HardDrive className="size-3.5" />
                {audit.efi_dir}
              </span>
              <Badge
                variant="outline"
                className={`shrink-0 gap-1 ${
                  audit.opencore
                    ? "border-[var(--success)]/40 text-[var(--success)]"
                    : "border-[var(--destructive)]/40 text-[var(--destructive)]"
                }`}
              >
                {audit.opencore ? (
                  <CircleCheck className="size-3" />
                ) : (
                  <CircleX className="size-3" />
                )}
                {audit.opencore ? t("efia.ocFound") : t("efia.ocMissing")}
              </Badge>
            </div>

            {audit.opencore && (
              <div className="text-muted-foreground">
                {t("efia.ocDetails", {
                  k: audit.kexts_count,
                  a: audit.acpi_count,
                  cfg: audit.has_config ? t("efia.yes") : t("efia.no"),
                })}
              </div>
            )}

            <div>
              <div className="text-muted-foreground mb-1.5">{t("efia.bootloaders")}</div>
              {audit.bootloaders.length ? (
                <div className="space-y-1">
                  {audit.bootloaders.map((b) => (
                    <div key={b.path} className="flex items-center gap-2">
                      {kindIcon(b.kind)}
                      <span className="font-medium">
                        {b.kind === "other" ? `${t("efia.kind.other")}: ${b.name}` : b.name}
                      </span>
                      <span className="text-muted-foreground font-mono text-[10px]">{b.path}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-muted-foreground italic">{t("efia.noBootloaders")}</div>
              )}
            </div>

            {audit.shared_bootx64 && (
              <div className="flex items-start gap-1.5 leading-relaxed text-muted-foreground">
                <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
                {t("efia.sharedBootx64")}
              </div>
            )}

            {audit.warnings.length > 0 && (
              <div className="space-y-1">
                <div className="text-[var(--destructive)] flex items-center gap-1.5 font-medium">
                  <TriangleAlert className="size-3.5" />
                  {t("efia.warnings")}
                </div>
                {audit.warnings.map((w, i) => (
                  <div key={i} className="text-[var(--destructive)] leading-relaxed">
                    • {tb(w)}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
