import { useCallback, useEffect, useState } from "react";
import { Cpu, CpuIcon, HardDrive, MemoryStick, MonitorSmartphone, Settings } from "lucide-react";
import { invoke } from "@/types";
import type { HardwareReport, KextEntry, MacosVersion, SmbiosData, SmbiosRecommendation, SsdtEntry } from "@/types";
import { useTheme } from "@/hooks/useTheme";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { SettingsDialog } from "@/components/SettingsDialog";
import { WelcomeStep } from "@/components/steps/WelcomeStep";
import { HardwareStep } from "@/components/steps/HardwareStep";
import { KextsStep } from "@/components/steps/KextsStep";
import { SmbiosStep } from "@/components/steps/SmbiosStep";
import { ConfigStep } from "@/components/steps/ConfigStep";
import { ExportStep } from "@/components/steps/ExportStep";

const STEPS = ["Начало", "Железо", "Кексты", "SMBIOS", "Конфиг", "Экспорт"];

export default function App() {
  const { dark, toggle } = useTheme();
  const [step, setStep] = useState(0);
  const [scanning, setScanning] = useState(false);
  const [report, setReport] = useState<HardwareReport | null>(null);
  const [darwin, setDarwin] = useState<number>(24);
  const [versions, setVersions] = useState<MacosVersion[]>([]);
  const [kexts, setKexts] = useState<KextEntry[]>([]);
  const [recommendation, setRecommendation] = useState<SmbiosRecommendation | null>(null);
  const [smbiosModel, setSmbiosModel] = useState<string>("");
  const [smbiosData, setSmbiosData] = useState<SmbiosData | null>(null);
  const [ssdts, setSsdts] = useState<SsdtEntry[]>([]);

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      const r = await invoke<HardwareReport>("scan_hardware");
      setReport(r);
      const max = await invoke<number>("compute_max_darwin", { report: r });
      setDarwin(Math.min(24, max));
      const vs = await invoke<MacosVersion[]>("macos_versions", { maxDarwin: max });
      setVersions(vs);
      const k = await invoke<KextEntry[]>("match_kexts", { report: r, darwin: Math.min(24, max) });
      setKexts(k);
      const rec = await invoke<SmbiosRecommendation>("recommend_smbios", { report: r, darwin: Math.min(24, max) });
      setRecommendation(rec);
      setSmbiosModel(rec.recommended);
      const s = await invoke<SsdtEntry[]>("recommend_ssdt", { report: r });
      setSsdts(s);
      setStep(1);
    } catch (e) {
      console.error(e);
    } finally {
      setScanning(false);
    }
  }, []);

  const changeDarwin = useCallback(
    async (value: number) => {
      if (!report) return;
      setDarwin(value);
      const k = await invoke<KextEntry[]>("match_kexts", { report, darwin: value });
      setKexts(k);
      const rec = await invoke<SmbiosRecommendation>("recommend_smbios", { report, darwin: value });
      setRecommendation(rec);
      setSmbiosModel(rec.recommended);
      setSmbiosData(null);
    },
    [report]
  );

  const toggleKext = useCallback((name: string) => {
    setKexts((prev) => prev.map((k) => (k.name === name ? { ...k, selected: !k.selected } : k)));
  }, []);

  const toggleSsdt = useCallback((name: string) => {
    setSsdts((prev) => prev.map((s) => (s.name === name ? { ...s, selected: !s.selected } : s)));
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "F5") e.preventDefault();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const canNext = step > 0 || !!report;

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <header className="flex items-center justify-between border-b px-5 py-3">
        <div className="flex items-center gap-2.5">
          <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <Cpu className="size-4.5" />
          </div>
          <div>
            <div className="text-sm font-semibold leading-tight">OpenEFI</div>
            <div className="text-muted-foreground text-[11px] leading-tight">
              сборка EFI для хакинтоша
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1">
          {STEPS.map((label, i) => (
            <div key={label} className="flex items-center">
              <button
                onClick={() => i <= step && setStep(i)}
                disabled={i > step}
                className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${
                  i === step
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-accent"
                }`}
              >
                {i + 1}. {label}
              </button>
              {i < STEPS.length - 1 && <Separator orientation="vertical" className="mx-0.5 h-3.5" />}
            </div>
          ))}
          <SettingsDialog>
            <Button variant="ghost" size="icon" title="Настройки">
              <Settings className="size-4" />
            </Button>
          </SettingsDialog>
          <Button variant="ghost" size="icon" onClick={toggle} title="Тема">
            {dark ? "☀️" : "🌙"}
          </Button>
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-4xl px-6 py-6">
          {step === 0 && <WelcomeStep scanning={scanning} onScan={scan} />}
          {step === 1 && report && (
            <HardwareStep
              report={report}
              versions={versions}
              darwin={darwin}
              onDarwinChange={changeDarwin}
            />
          )}
          {step === 2 && (
            <KextsStep kexts={kexts} onToggle={toggleKext} darwin={darwin} />
          )}
          {step === 3 && report && (
            <SmbiosStep
              report={report}
              darwin={darwin}
              recommendation={recommendation}
              model={smbiosModel}
              onModelChange={setSmbiosModel}
              data={smbiosData}
              onData={setSmbiosData}
            />
          )}
          {step === 4 && (
            <ConfigStep
              report={report!}
              ssdts={ssdts}
              onToggleSsdt={toggleSsdt}
              kexts={kexts}
              smbios={smbiosData}
              darwin={darwin}
            />
          )}
          {step === 5 && report && (
            <ExportStep
              report={report}
              kexts={kexts.filter((k) => k.selected)}
              smbios={smbiosData}
              darwin={darwin}
            />
          )}
        </div>
      </main>

      <footer className="flex items-center justify-between border-t px-5 py-3">
        <Button variant="ghost" disabled={step === 0} onClick={() => setStep((s) => Math.max(0, s - 1))}>
          Назад
        </Button>
        <div className="text-muted-foreground text-xs">
          {report && (
            <span className="flex items-center gap-3">
              <span className="flex items-center gap-1">
                <CpuIcon className="size-3.5" />
                {report.cpu.name || "CPU"}
              </span>
              <span className="flex items-center gap-1">
                <MemoryStick className="size-3.5" />
                {report.memory_gb} ГБ
              </span>
              <span className="flex items-center gap-1">
                <MonitorSmartphone className="size-3.5" />
                {report.platform}
              </span>
              <span className="flex items-center gap-1">
                <HardDrive className="size-3.5" />
                macOS {versions.find((v) => v.darwin === darwin)?.name ?? darwin}
              </span>
            </span>
          )}
        </div>
        {step < 5 ? (
          <Button disabled={!canNext} onClick={() => setStep((s) => Math.min(5, s + 1))}>
            Далее
          </Button>
        ) : (
          <Badge variant="secondary">Готово</Badge>
        )}
      </footer>
    </div>
  );
}
