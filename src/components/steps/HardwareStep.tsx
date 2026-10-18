import { AlertTriangle, Bluetooth, Cpu, MemoryStick, MonitorSmartphone, Network, Speaker, TriangleAlert } from "lucide-react";
import type { HardwareReport, MacosVersion } from "@/types";
import { useI18n } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface Props {
  report: HardwareReport;
  versions: MacosVersion[];
  darwin: number;
  onDarwinChange: (darwin: number) => void;
}

export function HardwareStep({ report, versions, darwin, onDarwinChange }: Props) {
  const { t, tb } = useI18n();
  const gpuCount = report.gpus.length || 0;

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Card>
          <CardHeader className="flex-row items-center gap-3 space-y-0">
            <div className="flex size-10 items-center justify-center rounded-lg bg-muted">
              <Cpu className="size-5" />
            </div>
            <div className="min-w-0">
              <CardTitle className="truncate text-sm">{report.cpu.name || t("hw.cpu")}</CardTitle>
              <CardDescription className="text-xs">
                {report.cpu.manufacturer}
                {report.cpu.codename ? ` · ${report.cpu.codename}` : ""}
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-1.5">
              <Badge variant="secondary">
                {t("hw.coresThreads", { cores: report.cpu.cores, threads: report.cpu.threads })}
              </Badge>
              <Badge variant={report.cpu.avx2 ? "secondary" : "warning"}>
                {report.cpu.avx2 ? "AVX2" : t("hw.noAvx2")}
              </Badge>
              <Badge variant="secondary">
                <MemoryStick className="size-3" />
                {report.memory_gb} {t("app.gb")}
              </Badge>
              <Badge variant="outline">{report.platform}</Badge>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-center gap-3 space-y-0">
            <div className="flex size-10 items-center justify-center rounded-lg bg-muted">
              <MonitorSmartphone className="size-5" />
            </div>
            <div className="min-w-0">
              <CardTitle className="text-sm">
                {t("hw.gpus")}
                {gpuCount > 0 ? ` · ${gpuCount}` : ""}
              </CardTitle>
              <CardDescription className="text-xs">{t("hw.gpusSubtitle")}</CardDescription>
            </div>
          </CardHeader>
          <CardContent className="space-y-2">
            {report.gpus.length === 0 && (
              <div className="text-muted-foreground text-xs">{t("hw.gpusEmpty")}</div>
            )}
            {report.gpus.map((g, i) => (
              <div key={i} className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="truncate text-xs font-medium">{g.name}</div>
                  <div className="text-muted-foreground text-[11px]">
                    {tb(g.device_type)} · {g.vendor_id}:{g.device_id}
                  </div>
                </div>
                {g.note && (
                  <Badge
                    variant={g.max_darwin === 0 ? "destructive" : "outline"}
                    className="shrink-0"
                    title={tb(g.note)}
                  >
                    {g.max_darwin === 0
                      ? t("hw.unsupported")
                      : g.max_darwin
                        ? `≤ Darwin ${g.max_darwin}`
                        : "OK"}
                  </Badge>
                )}
              </div>
            ))}
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        <DeviceCard
          title={t("hw.network")}
          icon={<Network className="size-5" />}
          devices={report.network}
        />
        <DeviceCard
          title={t("hw.audio")}
          icon={<Speaker className="size-5" />}
          devices={report.audio}
        />
        <DeviceCard
          title={t("hw.bluetooth")}
          icon={<Bluetooth className="size-5" />}
          devices={report.bluetooth}
        />
      </div>

      {report.warnings.length > 0 && (
        <div className="rounded-lg border border-[var(--warning)]/40 bg-[var(--warning)]/10 px-4 py-3">
          {report.warnings.map((w, i) => (
            <div key={i} className="flex items-center gap-2 text-xs">
              <AlertTriangle className="size-3.5 shrink-0" />
              {tb(w)}
            </div>
          ))}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">{t("hw.targetMacos")}</CardTitle>
          <CardDescription className="text-xs">{t("hw.targetHint")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            {versions.map((v) => (
              <button
                key={v.number}
                onClick={() => onDarwinChange(v.darwin)}
                className={cn(
                  "rounded-lg border px-3 py-2 text-left transition-colors",
                  v.darwin === darwin
                    ? "border-primary bg-primary text-primary-foreground"
                    : "hover:bg-accent",
                  !v.supported && "opacity-45"
                )}
              >
                <div className="text-sm font-semibold">macOS {v.number}</div>
                <div className="text-[11px] opacity-80">{v.name}</div>
              </button>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function DeviceCard({
  title,
  icon,
  devices,
}: {
  title: string;
  icon: React.ReactNode;
  devices: HardwareReport["network"];
}) {
  const { t } = useI18n();
  return (
    <Card>
      <CardHeader className="flex-row items-center gap-3 space-y-0">
        <div className="flex size-10 items-center justify-center rounded-lg bg-muted">{icon}</div>
        <div>
          <CardTitle className="text-sm">{title}</CardTitle>
          <CardDescription className="text-xs">{t("hw.devices", { n: devices.length })}</CardDescription>
        </div>
      </CardHeader>
      <CardContent className="space-y-1.5">
        {devices.length === 0 && (
          <div className="text-muted-foreground text-xs">{t("hw.none")}</div>
        )}
        {devices.map((d, i) => (
          <div key={i}>
            <div className="flex items-center gap-1.5">
              {d.note?.includes("не поддерживаются") && (
                <TriangleAlert className="size-3 shrink-0 text-[var(--warning)]" />
              )}
              <div className="truncate text-xs font-medium">{d.name}</div>
            </div>
            <div className="text-muted-foreground truncate text-[11px]">
              {d.kind}
              {d.vendor_id ? ` · ${d.vendor_id}:${d.device_id}` : ""}
            </div>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
