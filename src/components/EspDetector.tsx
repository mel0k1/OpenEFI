import { useState } from "react";
import { HardDrive, Loader2, MapPin, RotateCcw, Search } from "lucide-react";
import type { EfiPartition } from "@/types";
import { invoke } from "@/types";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  /** Найденная база EFI (том или каталог EFI) — поднимается в App и подставляется во все карточки */
  base: string | null;
  onBase: (base: string | null) => void;
}

/** Автопоиск EFI-раздела (ESP по GPT-типу) вместо ручного выбора папки */
export function EspDetector({ base, onBase }: Props) {
  const { t, tb } = useI18n();
  const [searching, setSearching] = useState(false);
  const [parts, setParts] = useState<EfiPartition[] | null>(null);
  const [mounting, setMounting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const find = async () => {
    setSearching(true);
    setError(null);
    setParts(null);
    try {
      const list = await invoke<EfiPartition[]>("find_esp_partitions");
      setParts(list);
      const mounted = list.filter((p) => p.mounted_at);
      if (mounted.length === 1) {
        // единственный смонтированный ESP — сразу используем
        onBase(mounted[0].mounted_at!);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  };

  const mount = async (device: string) => {
    setMounting(device);
    setError(null);
    try {
      const at = await invoke<string>("mount_esp_partition", { device });
      onBase(at);
      setParts((prev) =>
        (prev ?? []).map((p) => (p.device === device ? { ...p, mounted_at: at } : p)),
      );
    } catch (e) {
      setError(t("esp.mountFail", { error: String(e) }));
    } finally {
      setMounting(null);
    }
  };

  const mountedHere = parts?.filter((p) => p.mounted_at) ?? [];
  const unmountedHere = parts?.filter((p) => !p.mounted_at) ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <HardDrive className="size-4" />
          {t("esp.title")}
        </CardTitle>
        <CardDescription className="text-xs">{t("esp.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" size="sm" onClick={find} disabled={searching} className="gap-2">
            {searching ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <Search className="size-3.5" />
            )}
            {searching ? t("esp.searching") : t("esp.find")}
          </Button>
          {base && (
            <span className="flex min-w-0 items-center gap-1.5 text-xs">
              <MapPin className="text-[var(--success)] size-3.5 shrink-0" />
              <span className="truncate font-medium">{t("esp.using", { path: base })}</span>
              <button
                className="text-muted-foreground ml-1 shrink-0 hover:underline"
                onClick={() => onBase(null)}
              >
                ({t("esp.reset")})
              </button>
            </span>
          )}
        </div>

        {mountedHere.map((p) => (
          <button
            key={p.device}
            className="flex w-full items-center justify-between rounded-lg border px-3 py-2 text-left text-xs transition-colors hover:bg-accent/40"
            onClick={() => onBase(p.mounted_at!)}
          >
            <span className="font-mono">{p.device}</span>
            <span className="text-muted-foreground">
              {p.mounted_at}
              {p.size_mb != null && ` · ${p.size_mb} ${t("app.mb")}`}
            </span>
          </button>
        ))}

        {unmountedHere.map((p) => (
          <div
            key={p.device}
            className="flex items-center justify-between gap-2 rounded-lg border px-3 py-2 text-xs"
          >
            <span className="font-mono">{p.device}</span>
            <Button
              variant="secondary"
              size="sm"
              className="h-7 gap-1.5 text-[11px]"
              disabled={mounting === p.device}
              onClick={() => mount(p.device)}
            >
              {mounting === p.device ? (
                <Loader2 className="size-3 animate-spin" />
              ) : (
                <RotateCcw className="size-3" />
              )}
              {mounting === p.device ? t("esp.mounting") : t("esp.mount")}
            </Button>
          </div>
        ))}

        {parts !== null && parts.length === 0 && (
          <div className="text-muted-foreground text-xs">{t("esp.notFound")}</div>
        )}

        {error && <div className="text-destructive text-xs leading-relaxed">{tb(error)}</div>}
      </CardContent>
    </Card>
  );
}
