import { useState } from "react";
import { CheckCircle2, Download, ExternalLink, FileCode2, Loader2, ShieldAlert, TriangleAlert, Wrench } from "lucide-react";
import { invoke } from "@/types";
import type { Compensations, HardwareReport, KextEntry, SmbiosData, SsdtEntry } from "@/types";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { SsdtEfiInstall } from "@/components/SsdtEfiInstall";

interface Props {
  report: HardwareReport;
  ssdts: SsdtEntry[];
  onToggleSsdt: (name: string) => void;
  kexts: KextEntry[];
  smbios: SmbiosData | null;
  darwin: number;
  compensations: Compensations;
  /** Найденная ESP-база из детектора */
  base?: string | null;
}

const KIND_BADGE: Record<string, string> = {
  "Обязательна": "destructive",
  "Рекомендуется": "secondary",
  "Опциональна": "outline",
};

export function ConfigStep({ report, ssdts, onToggleSsdt, kexts, smbios, darwin, compensations, base }: Props) {
  const { t, tb } = useI18n();
  const [saving, setSaving] = useState(false);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedKexts = kexts.filter((k) => k.selected);
  const selectedSsdts = ssdts.filter((s) => s.selected);

  // OCLP-режим: целевая macOS выше нативного лимита хоть одной GPU → csr-active-config
  const sipOclp = report.gpus.some(
    (g) => g.max_darwin != null && g.max_darwin > 0 && g.max_darwin < darwin,
  );
  const anyCompensation = compensations.cfg_lock || compensations.vtd;

  const generate = async () => {
    if (!smbios) return;
    setSaving(true);
    setError(null);
    setSavedPath(null);
    try {
      const path = await invoke<string>("save_config_plist", {
        report,
        kexts: selectedKexts,
        ssdts: selectedSsdts,
        smbios,
        darwin,
        compensations,
      });
      setSavedPath(path);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-5">
      <Card>
        <CardHeader className="flex-row items-center gap-3 space-y-0">
          <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
            <FileCode2 className="size-5" />
          </div>
          <div>
            <CardTitle className="text-sm">{t("cfg.ssdtTitle")}</CardTitle>
            <CardDescription>{t("cfg.ssdtSubtitle")}</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground mb-3 text-xs leading-relaxed">
            {t("cfg.ssdtHintPre")}{" "}
            <button
              className="text-primary inline-flex items-center gap-0.5 hover:underline"
              onClick={() => openUrl("https://github.com/dortania/Getting-Started-With-ACPI/tree/master/extra-files/compiled")}
            >
              {t("cfg.ssdtHintLink")}
              <ExternalLink className="size-3" />
            </button>{" "}
            {t("cfg.ssdtHintPost")}
          </p>
          <div className="grid gap-2">
            {ssdts.map((s) => (
              <div
                key={s.name}
                className="flex items-start gap-3 rounded-lg border p-3 transition-colors hover:bg-accent/40"
              >
                <Checkbox checked={s.selected} disabled={s.kind === "Обязательна"} onCheckedChange={() => onToggleSsdt(s.name)} />
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-mono text-xs font-semibold">{s.file}</span>
                    <Badge variant={(KIND_BADGE[s.kind] ?? "outline") as never} className="text-[10px]">
                      {tb(s.kind)}
                    </Badge>
                  </div>
                  <p className="text-muted-foreground mt-0.5 text-xs leading-relaxed">{tb(s.description)}</p>
                </div>
              </div>
            ))}
          </div>

          <SsdtEfiInstall ssdts={ssdts} base={base} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-center gap-3 space-y-0">
          <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
            <Download className="size-5" />
          </div>
          <div>
            <CardTitle className="text-sm">{t("cfg.genTitle")}</CardTitle>
            <CardDescription>{t("cfg.genSubtitle")}</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {/* Компенсации BIOS → кварки */}
          <div className="rounded-lg border p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium">
              <Wrench className="text-primary size-3.5" />
              {t("cfg.quirksTitle")}
            </div>
            {anyCompensation ? (
              <div className="mt-2 flex flex-wrap gap-1.5">
                {compensations.cfg_lock && (
                  <Badge variant="secondary" className="text-[10px]">
                    {t("cfg.quirksCfgLock")}
                  </Badge>
                )}
                {compensations.vtd && (
                  <Badge variant="secondary" className="text-[10px]">
                    {t("cfg.quirksVtd")}
                  </Badge>
                )}
              </div>
            ) : (
              <p className="text-muted-foreground mt-1 text-xs">{t("cfg.quirksEmpty")}</p>
            )}
          </div>

          {/* SIP под OCLP */}
          {sipOclp && (
            <div className="flex items-start gap-2 rounded-md bg-amber-500/10 p-3 text-amber-600 dark:text-amber-400">
              <ShieldAlert className="mt-0.5 size-4 shrink-0" />
              <div className="text-xs leading-relaxed">
                <div className="font-medium">{t("cfg.sipTitle")}</div>
                <div className="mt-0.5">{t("cfg.sipText")}</div>
              </div>
            </div>
          )}

          <div className="grid gap-2 text-xs sm:grid-cols-2">
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">{t("cfg.acpiAdd")}</div>
              <p className="text-muted-foreground">
                {t("cfg.acpiSummary", { n: selectedSsdts.length })}
              </p>
            </div>
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">{t("cfg.kernelAdd")}</div>
              <p className="text-muted-foreground">
                {t("cfg.kernelSummary", { n: selectedKexts.filter((k) => !k.non_efi).length })}
              </p>
            </div>
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">{t("cfg.piGeneric")}</div>
              <p className="text-muted-foreground">
                {smbios ? t("cfg.piOk", { model: smbios.model }) : t("cfg.piEmpty")}
              </p>
            </div>
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">{t("cfg.dp")}</div>
              <p className="text-muted-foreground">
                {report.cpu.manufacturer === "Intel" &&
                report.gpus.some((g) => g.vendor_id === "8086" && g.device_type === "Integrated GPU")
                  ? t("cfg.dpOk")
                  : t("cfg.dpEmpty")}
              </p>
            </div>
          </div>

          <p className="text-muted-foreground text-xs leading-relaxed">
            {t("cfg.genNote")}
          </p>

          {!smbios && (
            <div className="flex items-start gap-2 rounded-md bg-amber-500/10 p-3 text-amber-600 dark:text-amber-400">
              <TriangleAlert className="mt-0.5 size-4 shrink-0" />
              <div className="text-xs leading-relaxed">
                {t("cfg.noSmbiosWarn")}
              </div>
            </div>
          )}

          {savedPath && (
            <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
              <div className="min-w-0 text-xs leading-relaxed">
                <div className="font-medium">{t("cfg.saved")}</div>
                <div className="truncate text-muted-foreground">{savedPath}</div>
              </div>
            </div>
          )}

          {error && (
            <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-destructive">
              <TriangleAlert className="mt-0.5 size-4 shrink-0" />
              <div className="text-xs leading-relaxed">{error}</div>
            </div>
          )}

          <Button className="w-full" disabled={!smbios || saving} onClick={generate}>
            {saving ? (
              <>
                <Loader2 className="size-4 animate-spin" />
                {t("cfg.generating")}
              </>
            ) : (
              <>
                <Download className="size-4" />
                {t("cfg.generate")}
              </>
            )}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
