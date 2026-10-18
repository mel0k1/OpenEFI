import { useMemo, useState } from "react";
import { Check, Download, ExternalLink, Loader2, Lock, Search } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { KextEntry } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { BACKEND_EN } from "@/i18n/backend";
import { Badge, type BadgeVariant } from "@/components/ui/badge-ext";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { EfiInstallCard } from "@/components/EfiInstallCard";
import { UsbMappingCard } from "@/components/UsbMappingCard";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

interface Props {
  kexts: KextEntry[];
  onToggle: (name: string) => void;
  darwin: number;
}

const STATUS_VARIANTS: Record<string, BadgeVariant> = {
  recommended: "default",
  dependency: "secondary",
  needs_attention: "warning",
  incompatible: "destructive",
};

export function KextsStep({ kexts, onToggle, darwin }: Props) {
  const { t, tb } = useI18n();
  const [query, setQuery] = useState("");

  const selected = kexts.filter((k) => k.selected);
  const categories = useMemo(() => {
    const cats = new Map<string, KextEntry[]>();
    for (const k of kexts) {
      if (!cats.has(k.category)) cats.set(k.category, []);
      cats.get(k.category)!.push(k);
    }
    return cats;
  }, [kexts]);

  const descEn = (k: KextEntry) => BACKEND_EN[k.description] ?? "";

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    return kexts.filter(
      (k) =>
        k.name.toLowerCase().includes(q) ||
        k.description.toLowerCase().includes(q) ||
        descEn(k).toLowerCase().includes(q) ||
        k.category.toLowerCase().includes(q) ||
        (BACKEND_EN[k.category] ?? "").toLowerCase().includes(q)
    );
  }, [kexts, query]);

  return (
    <div className="space-y-5">
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">
            {t("kexts.selectedCount", { n: selected.length })}
          </CardTitle>
          <CardDescription className="text-xs">
            {t("kexts.listHint", { darwin })}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-1.5">
            {selected.map((k) => (
              <Badge key={k.name} variant={STATUS_VARIANTS[k.status] ?? "secondary"}>
                {k.name}
              </Badge>
            ))}
          </div>
        </CardContent>
      </Card>

      <EfiInstallCard kexts={kexts} darwin={darwin} />

      <UsbMappingCard />

      <div className="relative">
        <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
        <Input
          placeholder={t("kexts.search")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="pl-9"
        />
      </div>

      <ScrollArea className="h-[420px] rounded-xl border">
        <div className="space-y-4 p-4">
          {[...categories.entries()]
            .filter(([cat]) => filtered.some((k) => k.category === cat))
            .map(([category, items]) => (
              <div key={category}>
                <div className="text-muted-foreground mb-2 text-xs font-semibold tracking-wide uppercase">
                  {tb(category)}
                </div>
                <div className="space-y-1.5">
                  {items
                    .filter((k) => filtered.includes(k))
                    .map((k) => (
                      <KextRow key={k.name} kext={k} onToggle={onToggle} />
                    ))}
                </div>
              </div>
            ))}
          {filtered.length === 0 && (
            <div className="text-muted-foreground py-8 text-center text-sm">
              {t("kexts.notFound")}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}

function KextRow({ kext, onToggle }: { kext: KextEntry; onToggle: (name: string) => void }) {
  const { t, tb } = useI18n();
  const status = kext.status;
  const locked = kext.required || kext.status === "dependency";
  const [dlState, setDlState] = useState<"idle" | "running" | "done">("idle");
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [dlError, setDlError] = useState<string | null>(null);

  const downloadDriver = async () => {
    if (!kext.repo) return;
    setDlState("running");
    setDlError(null);
    try {
      const path = await invoke<string>("download_driver_file", { repo: kext.repo, ext: ".dmg" });
      setSavedPath(path);
      setDlState("done");
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("отменено") && !msg.includes("отменён")) setDlError(msg);
      setDlState("idle");
    }
  };

  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-lg border px-3 py-2.5 transition-colors",
        kext.selected ? "bg-accent/50" : "hover:bg-accent/30",
        kext.status === "incompatible" && "opacity-55"
      )}
    >
      <Checkbox
        checked={kext.selected}
        disabled={locked || kext.status === "incompatible"}
        onCheckedChange={() => onToggle(kext.name)}
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{kext.name}</span>
          {kext.required && <Lock className="text-muted-foreground size-3" />}
          {kext.non_efi && (
            <Badge variant="outline" className="text-[10px]">
              {t("kexts.sysDriver")}
            </Badge>
          )}
          {status && (
            <Badge variant={STATUS_VARIANTS[status] ?? "secondary"} className="text-[10px]">
              {t(`kexts.status.${status}`)}
            </Badge>
          )}
        </div>
        <div className="text-muted-foreground truncate text-xs">{tb(kext.description)}</div>
        {dlState === "done" && savedPath && (
          <div className="mt-1 flex items-center gap-1 text-[11px] text-emerald-600 dark:text-emerald-400">
            <Check className="size-3 shrink-0" />
            <span className="truncate">{t("kexts.driverSaved", { path: savedPath })}</span>
          </div>
        )}
        {dlError && (
          <div className="mt-1 text-[11px] text-destructive">{dlError}</div>
        )}
      </div>
      {kext.non_efi && kext.repo && (
        <button
          onClick={downloadDriver}
          disabled={dlState === "running"}
          className="text-muted-foreground hover:text-foreground shrink-0 disabled:opacity-50"
          title={dlState === "running" ? t("kexts.driverDownloading") : t("kexts.downloadDriver")}
        >
          {dlState === "running" ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <Download className="size-4" />
          )}
        </button>
      )}
      {kext.repo && (
        <button
          onClick={() => openUrl(`https://github.com/${kext.repo}`)}
          className="text-muted-foreground hover:text-foreground shrink-0"
          title={t("kexts.repoTitle", { repo: kext.repo })}
        >
          <ExternalLink className="size-4" />
        </button>
      )}
    </div>
  );
}
