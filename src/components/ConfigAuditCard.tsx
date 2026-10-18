import { useState } from "react";
import {
  CircleAlert,
  CircleCheck,
  CircleHelp,
  FileCode2,
  Loader2,
  PackageSearch,
} from "lucide-react";
import type { AuditIssue, ConfigAudit } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  /** Найденная ESP-база из детектора (undefined — диалог выбора папки) */
  base?: string | null;
}

/** Аудит существующего config.plist: сверка Kernel/Add и ACPI/Add с файлами на диске */
export function ConfigAuditCard({ base }: Props) {
  const { t, tb } = useI18n();
  const [audit, setAudit] = useState<ConfigAudit | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setChecking(true);
    setError(null);
    setAudit(null);
    try {
      setAudit(await invoke<ConfigAudit>("audit_config_plist", { base: base ?? undefined }));
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменён")) setError(msg);
    } finally {
      setChecking(false);
    }
  };

  const fmtIssue = (i: AuditIssue) => {
    let s = t(`cfga.issue.${i.code}`);
    i.params.forEach((p, idx) => {
      s = s.split(`{${idx}}`).join(p);
    });
    return s;
  };

  const severityIcon = (sev: string) =>
    sev === "error" ? (
      <CircleAlert className="text-destructive mt-0.5 size-3.5 shrink-0" />
    ) : sev === "warning" ? (
      <CircleAlert className="text-amber-500 mt-0.5 size-3.5 shrink-0" />
    ) : (
      <CircleHelp className="text-primary mt-0.5 size-3.5 shrink-0" />
    );

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <FileCode2 className="size-4" />
          {t("cfga.title")}
        </CardTitle>
        <CardDescription className="text-xs">{t("cfga.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <Button variant="outline" size="sm" onClick={run} disabled={checking} className="gap-2">
          {checking ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <PackageSearch className="size-3.5" />
          )}
          {checking ? t("cfga.checking") : t("cfga.check")}
        </Button>

        {audit && !audit.found && (
          <div className="flex items-start gap-2 rounded-md bg-amber-500/10 p-3 text-amber-600 dark:text-amber-400">
            <CircleAlert className="mt-0.5 size-4 shrink-0" />
            <div className="text-xs leading-relaxed">{t("cfga.notFound")}</div>
          </div>
        )}

        {audit?.found && (
          <div className="space-y-3">
            <div className="text-muted-foreground text-xs">
              {t("cfga.summary", {
                k: audit.kexts_total,
                e: audit.kexts_enabled,
                a: audit.tables_total,
              })}
            </div>

            {audit.issues.length === 0 ? (
              <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
                <CircleCheck className="mt-0.5 size-4 shrink-0" />
                <div className="text-xs leading-relaxed">{t("cfga.ok")}</div>
              </div>
            ) : (
              <div className="space-y-1.5">
                <div className="text-xs font-medium">{t("cfga.issues")}</div>
                {audit.issues.map((i, idx) => (
                  <div key={idx} className="flex items-start gap-2 text-xs leading-relaxed">
                    {severityIcon(i.severity)}
                    <span>
                      <Badge
                        variant="outline"
                        className={`mr-1.5 text-[10px] ${
                          i.severity === "error"
                            ? "border-destructive/40 text-destructive"
                            : i.severity === "warning"
                              ? "border-amber-500/40 text-amber-600 dark:text-amber-400"
                              : "border-primary/40 text-primary"
                        }`}
                      >
                        {t(`cfga.severity.${i.severity}`)}
                      </Badge>
                      {fmtIssue(i)}
                    </span>
                  </div>
                ))}
              </div>
            )}

            {audit.kernel.length > 0 && (
              <div className="space-y-1">
                <div className="text-muted-foreground text-xs font-medium">{t("cfga.kexts")}</div>
                {audit.kernel.map((k) => (
                  <div
                    key={k.bundle}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2 text-xs"
                  >
                    <span className="font-mono">{k.bundle}</span>
                    <span className="flex items-center gap-1.5">
                      {k.version && <Badge variant="secondary" className="text-[10px]">{k.version}</Badge>}
                      {k.enabled ? (
                        <CircleCheck className="text-[var(--success)] size-3.5" />
                      ) : (
                        <Badge variant="outline" className="text-[10px]">{t("cfga.disabled")}</Badge>
                      )}
                      {!k.on_disk && <CircleAlert className="text-destructive size-3.5" />}
                    </span>
                  </div>
                ))}
              </div>
            )}

            {audit.acpi.length > 0 && (
              <div className="space-y-1">
                <div className="text-muted-foreground text-xs font-medium">{t("cfga.acpi")}</div>
                {audit.acpi.map((a) => (
                  <div
                    key={a.path}
                    className="flex items-center justify-between gap-2 rounded-lg border px-3 py-2 text-xs"
                  >
                    <span className="font-mono">{a.path}</span>
                    <span className="flex items-center gap-1.5">
                      {a.enabled ? (
                        <CircleCheck className="text-[var(--success)] size-3.5" />
                      ) : (
                        <Badge variant="outline" className="text-[10px]">{t("cfga.disabled")}</Badge>
                      )}
                      {!a.on_disk && <CircleAlert className="text-destructive size-3.5" />}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {error && <div className="text-destructive text-xs leading-relaxed">{tb(error)}</div>}
      </CardContent>
    </Card>
  );
}
