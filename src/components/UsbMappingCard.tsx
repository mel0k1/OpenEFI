import { useState } from "react";
import { AlertCircle, CheckCircle2, Download, Loader2, PlugZap } from "lucide-react";
import type { UtbMapReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type ToolTarget = "windows" | "macos";

interface Props {
  /** Найденная ESP-база из детектора (undefined — диалог выбора папки) */
  base?: string | null;
}

export function UsbMappingCard({ base }: Props) {
  const { t, tb } = useI18n();
  const [toolBusy, setToolBusy] = useState<ToolTarget | null>(null);
  const [toolPath, setToolPath] = useState<string | null>(null);
  const [toolError, setToolError] = useState<string | null>(null);

  const [adding, setAdding] = useState(false);
  const [report, setReport] = useState<UtbMapReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const downloadTool = async (target: ToolTarget) => {
    setToolBusy(target);
    setToolError(null);
    try {
      const path = await invoke<string>("download_utb_tool", { target });
      setToolPath(path);
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменено") && !msg.includes("отменён")) setToolError(msg);
    } finally {
      setToolBusy(null);
    }
  };

  const addUtbMap = async () => {
    setAdding(true);
    setError(null);
    try {
      const r = await invoke<UtbMapReport>("install_utbmap_to_efi", { base: base ?? undefined });
      setReport(r);
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменён")) setError(msg);
    } finally {
      setAdding(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <PlugZap className="size-4" />
          {t("usb.title")}
        </CardTitle>
        <CardDescription className="text-xs">{t("usb.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <ol className="space-y-1.5 text-xs leading-relaxed">
          {[t("usb.step1"), t("usb.step2"), t("usb.step3")].map((s, i) => (
            <li key={i} className="flex gap-2">
              <span className="bg-primary/10 text-primary flex size-4.5 shrink-0 items-center justify-center rounded-full text-[10px] font-bold">
                {i + 1}
              </span>
              {s}
            </li>
          ))}
        </ol>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => downloadTool("windows")}
            disabled={toolBusy !== null}
            className="gap-1.5"
          >
            {toolBusy === "windows" ? <Loader2 className="size-3.5 animate-spin" /> : <Download className="size-3.5" />}
            {toolBusy === "windows" ? t("usb.toolDownloading") : t("usb.toolWin")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => downloadTool("macos")}
            disabled={toolBusy !== null}
            className="gap-1.5"
          >
            {toolBusy === "macos" ? <Loader2 className="size-3.5 animate-spin" /> : <Download className="size-3.5" />}
            {toolBusy === "macos" ? t("usb.toolDownloading") : t("usb.toolMac")}
          </Button>
          <Button onClick={addUtbMap} disabled={adding} size="sm" className="gap-1.5">
            {adding ? <Loader2 className="size-3.5 animate-spin" /> : <PlugZap className="size-3.5" />}
            {adding ? t("usb.adding") : t("usb.addMap")}
          </Button>
        </div>

        {toolPath && (
          <div className="flex items-center gap-1.5 text-emerald-600 dark:text-emerald-400 text-xs">
            <CheckCircle2 className="size-3.5 shrink-0" />
            <span className="break-all">{t("usb.toolSaved", { path: toolPath })}</span>
          </div>
        )}

        {toolError && (
          <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-destructive">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <div className="text-xs leading-relaxed">{toolError}</div>
          </div>
        )}

        {report && (
          <div className="space-y-1.5">
            <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
              <div className="text-xs leading-relaxed">
                <div className="font-medium">{t("usb.mapDone", { dir: report.kexts_dir })}</div>
                {report.config_path ? (
                  <>
                    <div className="text-muted-foreground">{t("usb.configPatched")}</div>
                    {report.backup_made && (
                      <div className="text-muted-foreground break-all">
                        {t("usb.configBackup", { path: `${report.config_path}.bak-openefi` })}
                      </div>
                    )}
                  </>
                ) : (
                  <div className="text-muted-foreground">{t("usb.configMissing")}</div>
                )}
              </div>
            </div>
            <div className="text-muted-foreground text-xs">{t("usb.deleteHint")}</div>
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
