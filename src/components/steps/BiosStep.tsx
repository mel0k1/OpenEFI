import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Check,
  CircleAlert,
  CircleCheck,
  CircleHelp,
  RefreshCw,
  Search,
  ShieldCheck,
  TriangleAlert,
  Wrench,
  X,
} from "lucide-react";
import type { BiosChecklist, BiosItem, BiosStatus, Compensations, HardwareReport } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";

interface Props {
  report: HardwareReport;
  compensations: Compensations;
  onCompensation: (key: "cfg_lock" | "vtd", value: boolean) => void;
}

type Group = "required" | "recommended" | "optional";

export function BiosStep({ report, compensations, onCompensation }: Props) {
  const { t, tb } = useI18n();
  const [checklist, setChecklist] = useState<BiosChecklist | null>(null);
  const [status, setStatus] = useState<BiosStatus | null>(null);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    invoke<BiosChecklist>("bios_checklist", { report }).then(setChecklist).catch(console.error);
  }, [report]);

  const runCheck = async () => {
    setChecking(true);
    try {
      setStatus(await invoke<BiosStatus>("bios_live_status"));
    } catch (e) {
      console.error(e);
    } finally {
      setChecking(false);
    }
  };

  const groups: Group[] = ["required", "recommended", "optional"];

  const renderStatusBadge = (item: BiosItem) => (
    <Badge
      variant="outline"
      className={`gap-1 shrink-0 ${
        item.action === "enable"
          ? "border-[var(--success)]/40 text-[var(--success)]"
          : item.action === "disable"
            ? "border-[var(--destructive)]/40 text-[var(--destructive)]"
            : "border-primary/40 text-primary"
      }`}
    >
      {item.action === "enable" ? (
        <CircleCheck className="size-3" />
      ) : item.action === "disable" ? (
        <CircleAlert className="size-3" />
      ) : (
        <Search className="size-3" />
      )}
      {t(`bios.action.${item.action}`)}
    </Badge>
  );

  /** Кварковые компенсации под пункт BIOS: не нашёл CFG Lock / оставил VT-d */
  const compensationFor = (id: string): "cfg_lock" | "vtd" | null => {
    if (id === "cfg_lock") return "cfg_lock";
    if (id === "vt_d" || id === "iommu") return "vtd";
    return null;
  };

  const renderCompensation = (item: BiosItem) => {
    const key = compensationFor(item.id);
    if (!key) return null;
    const i18nKey = key === "cfg_lock" ? "cfgLock" : "vtd";
    const active = compensations[key];
    return (
      <div
        className={`mt-2 flex items-start gap-2.5 rounded-lg border p-2.5 ${
          active ? "border-primary/50 bg-primary/5" : ""
        }`}
      >
        <Checkbox
          id={`comp-${item.id}`}
          checked={active}
          onCheckedChange={(v) => onCompensation(key, v === true)}
        />
        <div className="min-w-0 flex-1">
          <Label
            htmlFor={`comp-${item.id}`}
            className="flex items-center gap-1.5 text-xs font-medium"
          >
            <Wrench className="text-primary size-3 shrink-0" />
            {t(`bios.compensate.${i18nKey}.title`)}
          </Label>
          <p className="text-muted-foreground mt-0.5 text-xs leading-relaxed">
            {t(`bios.compensate.${i18nKey}.desc`)}
          </p>
          {active && (
            <p className="text-primary mt-1 flex items-center gap-1 text-xs font-medium">
              <CircleCheck className="size-3 shrink-0" />
              {t(`bios.compensate.${i18nKey}.on`)}
            </p>
          )}
        </div>
      </div>
    );
  };

  const renderItem = (item: BiosItem) => (
    <div key={item.id} className="flex items-start gap-3 border-b py-2.5 last:border-0">
      <div className="mt-0.5 shrink-0">
        {item.priority === "required" ? (
          <span className="text-[var(--destructive)]">
            <TriangleAlert className="size-4" />
          </span>
        ) : (
          <span className="text-muted-foreground">
            <CircleHelp className="size-4" />
          </span>
        )}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-mono text-xs font-semibold">{item.option}</span>
          {renderStatusBadge(item)}
        </div>
        <div className="text-muted-foreground mt-1 text-xs leading-relaxed">
          {t(`bios.item.${item.id}.desc`)}
        </div>
        {item.note && (
          <div className="mt-1 text-xs leading-relaxed text-muted-foreground/80 italic">
            {tb(item.note)}
          </div>
        )}
        {renderCompensation(item)}
      </div>
    </div>
  );

  return (
    <div className="space-y-5">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <ShieldCheck className="size-4" />
            {t("bios.title")}
          </CardTitle>
          <CardDescription className="text-xs">{t("bios.desc")}</CardDescription>
        </CardHeader>
      </Card>

      {/* Живая проверка */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">{t("bios.live.title")}</CardTitle>
          <CardDescription className="text-xs">{t("bios.live.desc")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Button variant="outline" size="sm" onClick={runCheck} disabled={checking} className="gap-2">
            {checking ? (
              <RefreshCw className="size-3.5 animate-spin" />
            ) : (
              <Search className="size-3.5" />
            )}
            {checking ? t("bios.live.checking") : t("bios.live.check")}
          </Button>

          {status && (
            <div className="space-y-2">
              <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                <StatusRow
                  label={t("bios.live.secureBoot")}
                  value={
                    status.secure_boot === null
                      ? t("bios.live.unknown")
                      : status.secure_boot
                        ? t("bios.live.on")
                        : t("bios.live.off")
                  }
                  ok={status.secure_boot === false}
                  bad={status.secure_boot === true}
                />
                <StatusRow
                  label={t("bios.live.bootMode")}
                  value={
                    status.boot_mode === "uefi"
                      ? t("bios.live.uefi")
                      : status.boot_mode === "legacy"
                        ? t("bios.live.legacy")
                        : t("bios.live.unknown")
                  }
                  ok={status.boot_mode === "uefi"}
                  bad={status.boot_mode === "legacy"}
                />
              </div>
              {status.boot_mode === "legacy" && (
                <WarnNote text={t("bios.live.legacyWarn")} />
              )}
              {status.boot_mode === "uefi" && <OkNote text={t("bios.live.uefiOk")} />}
              {status.secure_boot === true && <WarnNote text={t("bios.live.secureWarn")} />}
              {status.secure_boot === false && <OkNote text={t("bios.live.secureOk")} />}
              {status.notes.map((n, i) => (
                <div key={i} className="text-muted-foreground text-xs italic">
                  {tb(n)}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Группы чеклиста */}
      {checklist &&
        groups.map((g) => {
          const items = checklist.items.filter((i) => i.priority === g);
          if (!items.length) return null;
          return (
            <Card key={g}>
              <CardHeader>
                <CardTitle className="text-sm">{t(`bios.${g}`)}</CardTitle>
              </CardHeader>
              <CardContent>{items.map(renderItem)}</CardContent>
            </Card>
          );
        })}

      <div className="text-muted-foreground space-y-1 text-xs">
        <div className="flex items-start gap-1.5">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{t("bios.hint")}</span>
        </div>
        <div className="flex items-start gap-1.5">
          <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
          <span>{t("bios.quirksHint")}</span>
        </div>
      </div>
    </div>
  );
}

function StatusRow({
  label,
  value,
  ok,
  bad,
}: {
  label: string;
  value: string;
  ok: boolean;
  bad: boolean;
}) {
  return (
    <div className="flex items-center justify-between rounded-lg border px-3 py-2.5">
      <span className="text-muted-foreground">{label}</span>
      <span
        className={`flex items-center gap-1.5 font-medium ${
          ok ? "text-[var(--success)]" : bad ? "text-[var(--destructive)]" : ""
        }`}
      >
        {ok ? (
          <Check className="size-3.5" />
        ) : bad ? (
          <X className="size-3.5" />
        ) : (
          <CircleHelp className="size-3.5" />
        )}
        {value}
      </span>
    </div>
  );
}

function WarnNote({ text }: { text: string }) {
  return (
    <div className="text-xs leading-relaxed text-[var(--destructive)]">
      <span className="flex items-start gap-1.5">
        <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
        {text}
      </span>
    </div>
  );
}

function OkNote({ text }: { text: string }) {
  return (
    <div className="text-xs leading-relaxed text-[var(--success)]">
      <span className="flex items-start gap-1.5">
        <CircleCheck className="mt-0.5 size-3.5 shrink-0" />
        {text}
      </span>
    </div>
  );
}
