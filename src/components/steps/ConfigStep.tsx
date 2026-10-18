import { useState } from "react";
import { CheckCircle2, Download, ExternalLink, FileCode2, Loader2, TriangleAlert } from "lucide-react";
import { invoke } from "@/types";
import type { HardwareReport, KextEntry, SmbiosData, SsdtEntry } from "@/types";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";

interface Props {
  report: HardwareReport;
  ssdts: SsdtEntry[];
  onToggleSsdt: (name: string) => void;
  kexts: KextEntry[];
  smbios: SmbiosData | null;
  darwin: number;
}

const KIND_BADGE: Record<string, string> = {
  "Обязательна": "destructive",
  "Рекомендуется": "secondary",
  "Опциональна": "outline",
};

export function ConfigStep({ report, ssdts, onToggleSsdt, kexts, smbios, darwin }: Props) {
  const [saving, setSaving] = useState(false);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedKexts = kexts.filter((k) => k.selected);
  const selectedSsdts = ssdts.filter((s) => s.selected);

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
            <CardTitle className="text-sm">Подбор SSDT (Dortania)</CardTitle>
            <CardDescription>
              Таблицы ACPI для вашей платформы — положите их в EFI/OC/ACPI
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground mb-3 text-xs leading-relaxed">
            Скомпилированные файлы берите из{" "}
            <button
              className="text-primary inline-flex items-center gap-0.5 hover:underline"
              onClick={() => openUrl("https://github.com/dortania/Getting-Started-With-ACPI/tree/master/extra-files/compiled")}
            >
              репозитория Dortania
              <ExternalLink className="size-3" />
            </button>{" "}
            — они помечены как precompiled. Файлы попадут в секцию ACPI/Add конфига.
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
                      {s.kind}
                    </Badge>
                  </div>
                  <p className="text-muted-foreground mt-0.5 text-xs leading-relaxed">{s.description}</p>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-center gap-3 space-y-0">
          <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
            <Download className="size-5" />
          </div>
          <div>
            <CardTitle className="text-sm">Генерация config.plist</CardTitle>
            <CardDescription>
              Эталон OpenCore + ваши кексты, SSDT и SMBIOS
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid gap-2 text-xs sm:grid-cols-2">
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">ACPI / Add</div>
              <p className="text-muted-foreground">
                {selectedSsdts.length} таблиц SSDT (Enabled)
              </p>
            </div>
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">Kernel / Add</div>
              <p className="text-muted-foreground">
                {selectedKexts.length} кекстов в правильном порядке (Lilu первым), MinKernel/MaxKernel
              </p>
            </div>
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">PlatformInfo / Generic</div>
              <p className="text-muted-foreground">
                {smbios ? `${smbios.model}: серийник, MLB, ROM, UUID` : "нет SMBIOS"}
              </p>
            </div>
            <div className="rounded-lg border p-2.5">
              <div className="font-medium">DeviceProperties</div>
              <p className="text-muted-foreground">
                {report.cpu.manufacturer === "Intel" &&
                report.gpus.some((g) => g.vendor_id === "8086" && g.device_type === "Integrated GPU")
                  ? "AAPL,ig-platform-id для вашего iGPU"
                  : "пусто — интегрированной графики Intel не найдено"}
              </p>
            </div>
          </div>

          <p className="text-muted-foreground text-xs leading-relaxed">
            Конфиг собирается на базе эталонного Sample.plist из OpenCorePkg, поэтому содержит все
            секции целиком. boot-args: <code className="font-mono">-v keepsyms=1</code>, XhciPortLimit
            выключен (darwin ≥ 20). SSDT .aml файлы нужно скачать отдельно и положить рядом с конфигом в
            EFI/OC/ACPI.
          </p>

          {!smbios && (
            <div className="flex items-start gap-2 rounded-md bg-amber-500/10 p-3 text-amber-600 dark:text-amber-400">
              <TriangleAlert className="mt-0.5 size-4 shrink-0" />
              <div className="text-xs leading-relaxed">
                Сначала сгенерируйте серийники на шаге SMBIOS — без них PlatformInfo конфига будет
                неполным.
              </div>
            </div>
          )}

          {savedPath && (
            <div className="flex items-start gap-2 rounded-md bg-emerald-500/10 p-3 text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
              <div className="min-w-0 text-xs leading-relaxed">
                <div className="font-medium">config.plist сохранён</div>
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
                Генерируем…
              </>
            ) : (
              <>
                <Download className="size-4" />
                Сгенерировать config.plist
              </>
            )}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
