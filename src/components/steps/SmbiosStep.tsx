import { useEffect, useState } from "react";
import { Fingerprint, Loader2, RefreshCw, Sparkles } from "lucide-react";
import { invoke } from "@/types";
import type { HardwareReport, MacDevice, SmbiosData, SmbiosRecommendation } from "@/types";
import { useCopy } from "@/hooks/useCopy";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Check, Copy } from "lucide-react";

interface Props {
  report: HardwareReport;
  darwin: number;
  recommendation: SmbiosRecommendation | null;
  model: string;
  onModelChange: (model: string) => void;
  data: SmbiosData | null;
  onData: (data: SmbiosData | null) => void;
}

export function SmbiosStep({
  report,
  darwin,
  recommendation,
  model,
  onModelChange,
  data,
  onData,
}: Props) {
  const { t, tb } = useI18n();
  const [models, setModels] = useState<MacDevice[]>([]);
  const [generating, setGenerating] = useState(false);
  const { copied, copy } = useCopy();

  useEffect(() => {
    invoke<MacDevice[]>("list_smbios_models", {
      darwin,
      isLaptop: report.platform.includes("Laptop"),
    }).then(setModels);
  }, [darwin, report.platform]);

  const generate = async () => {
    setGenerating(true);
    try {
      const d = await invoke<SmbiosData>("generate_smbios", { model });
      onData(d);
    } finally {
      setGenerating(false);
    }
  };

  return (
    <div className="space-y-5">
      {recommendation && (
        <Card className="border-primary/40">
          <CardHeader className="flex-row items-center gap-3 space-y-0">
            <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
              <Sparkles className="size-5" />
            </div>
            <div>
              <CardTitle className="text-sm">
                {t("smbios.recommendation", { model: recommendation.recommended })}
              </CardTitle>
              <CardDescription className="text-xs">{tb(recommendation.reason)}</CardDescription>
            </div>
          </CardHeader>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">{t("smbios.modelTitle")}</CardTitle>
          <CardDescription className="text-xs">
            {t("smbios.compatible", { darwin, platform: report.platform })}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="max-h-52 overflow-y-auto rounded-lg border p-2">
            <div className="grid grid-cols-2 gap-1 sm:grid-cols-3">
              {models.map((m) => (
                <button
                  key={m.name}
                  onClick={() => {
                    onModelChange(m.name);
                    onData(null);
                  }}
                  className={`rounded-md px-2.5 py-1.5 text-left text-xs transition-colors ${
                    model === m.name
                      ? "bg-primary text-primary-foreground"
                      : "hover:bg-accent"
                  }`}
                >
                  <div className="font-semibold">{m.name}</div>
                  <div className="opacity-75">{m.cpu}</div>
                </button>
              ))}
              {models.length === 0 && (
                <div className="text-muted-foreground col-span-3 py-4 text-center text-xs">
                  {t("smbios.loading")}
                </div>
              )}
            </div>
          </div>
          <Button onClick={generate} disabled={!model || generating} className="gap-2">
            {generating ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <RefreshCw className="size-4" />
            )}
            {data ? t("smbios.regenerate") : t("smbios.generate")}
          </Button>
        </CardContent>
      </Card>

      {data && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Fingerprint className="size-4" />
              {t("smbios.dataTitle", { model: data.model })}
            </CardTitle>
            <CardDescription className="text-xs">
              {t("smbios.dataHint")}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <Field label="SystemProductName" value={data.model} onCopy={copy} copied={copied} />
            <Field label="SystemSerialNumber" value={data.serial} onCopy={copy} copied={copied} />
            <Field label="MLB" value={data.mlb} onCopy={copy} copied={copied} />
            <Field label="ROM" value={data.rom} onCopy={copy} copied={copied} />
            <Field label="SystemUUID" value={data.uuid} onCopy={copy} copied={copied} />
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function Field({
  label,
  value,
  onCopy,
  copied,
}: {
  label: string;
  value: string;
  onCopy: (text: string, key: string) => void;
  copied: string | null;
}) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2">
      <div className="min-w-0">
        <div className="text-muted-foreground text-[11px]">{label}</div>
        <div className="truncate font-mono text-sm">{value}</div>
      </div>
      <Button variant="ghost" size="icon" onClick={() => onCopy(value, label)}>
        {copied === label ? <Check className="size-4" /> : <Copy className="size-4" />}
      </Button>
    </div>
  );
}
