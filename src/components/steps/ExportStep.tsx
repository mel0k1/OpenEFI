import { useState } from "react";
import { Check, Download, ExternalLink, FileJson, Loader2 } from "lucide-react";
import type { HardwareReport, KextEntry, SmbiosData } from "@/types";
import { invoke } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  report: HardwareReport;
  kexts: KextEntry[];
  smbios: SmbiosData | null;
  darwin: number;
}

export function ExportStep({ report, kexts, smbios, darwin }: Props) {
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const exportReport = async () => {
    setSaving(true);
    setError(null);
    try {
      const path = await invoke<string>("export_report", {
        report,
        kexts: kexts.map((k) => k.name),
        smbios,
      });
      setSaved(path);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-5">
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Итоговая конфигурация</CardTitle>
          <CardDescription className="text-xs">
            Сводка по вашему железу, целевой macOS и подобранным кекстам
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3 text-xs">
          <SummaryRow label="Процессор" value={`${report.cpu.name} (${report.cpu.codename ?? "?"})`} />
          <SummaryRow label="Платформа" value={`${report.platform} · ${report.board_name}`} />
          <SummaryRow
            label="Видеокарты"
            value={report.gpus.map((g) => g.name).join(", ") || "нет"}
          />
          <SummaryRow
            label="macOS"
            value={`Darwin ${darwin}`}
          />
          <div>
            <div className="text-muted-foreground mb-1.5">
              Кексты ({kexts.length}):
            </div>
            <div className="flex flex-wrap gap-1.5">
              {kexts.map((k) => (
                <Badge key={k.name} variant="secondary">
                  {k.name}
                </Badge>
              ))}
            </div>
          </div>
          {smbios && (
            <div className="grid grid-cols-2 gap-2 rounded-lg border p-3 font-mono text-[11px]">
              <div>
                <span className="text-muted-foreground">Serial: </span>
                {smbios.serial}
              </div>
              <div>
                <span className="text-muted-foreground">MLB: </span>
                {smbios.mlb}
              </div>
              <div>
                <span className="text-muted-foreground">ROM: </span>
                {smbios.rom}
              </div>
              <div>
                <span className="text-muted-foreground">UUID: </span>
                {smbios.uuid}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Что дальше</CardTitle>
          <CardDescription className="text-xs">
            Перед установкой macOS выполните эти шаги
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ol className="space-y-2 text-xs leading-relaxed">
            {[
              "Скачайте OpenCorePkg и соберите базовую EFI-папку (Utilities → BootInstall).",
              "Скачайте кексты из списка выше (кнопки-ссылки на шаге «Кексты») и положите в EFI/OC/Kexts.",
              "Настройте config.plist: ACPI, Kernel → Add (снапшот через ProperTree), PlatformInfo с данными SMBIOS.",
              "Замапьте USB-порты утилитой USBToolBox и замените UTBDefault на свой UTBMap.",
              "В BIOS включите UEFI, Above 4G Decoding, отключите Secure Boot и Resizable BAR.",
              "Проверьте отчёт: сохраните JSON ниже — он пригодится при отладке и на форумах.",
            ].map((step, i) => (
              <li key={i} className="flex gap-2">
                <span className="bg-primary/10 text-primary flex size-5 shrink-0 items-center justify-center rounded-full text-[10px] font-bold">
                  {i + 1}
                </span>
                {step}
              </li>
            ))}
          </ol>
          <div className="mt-4 flex gap-2">
            <Button onClick={exportReport} disabled={saving} className="gap-2">
              {saving ? (
                <Loader2 className="size-4 animate-spin" />
              ) : saved ? (
                <Check className="size-4" />
              ) : (
                <FileJson className="size-4" />
              )}
              {saved ? "Сохранить ещё раз" : "Сохранить отчёт JSON"}
            </Button>
            <Button variant="outline" className="gap-2" asChild>
              <a
                href="https://dortania.github.io/OpenCore-Install-Guide/"
                target="_blank"
                rel="noreferrer"
              >
                <ExternalLink className="size-4" />
                Гайд Dortania
              </a>
            </Button>
          </div>
          {saved && (
            <div className="mt-3 flex items-center gap-2 text-xs text-[var(--success)]">
              <Download className="size-3.5" />
              Отчёт сохранён: {saved}
            </div>
          )}
          {error && (
            <div className="mt-3 text-xs text-[var(--destructive)]">
              Ошибка сохранения: {error}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-4 border-b pb-2 last:border-0">
      <span className="text-muted-foreground shrink-0">{label}</span>
      <span className="text-right font-medium">{value}</span>
    </div>
  );
}
