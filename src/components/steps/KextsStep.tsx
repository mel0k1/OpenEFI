import { useMemo, useState } from "react";
import { ExternalLink, Lock, Search } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { KextEntry } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

interface Props {
  kexts: KextEntry[];
  onToggle: (name: string) => void;
  darwin: number;
}

const STATUS_LABELS: Record<string, { label: string; variant: "default" | "secondary" | "destructive" | "warning" }> = {
  recommended: { label: "рекомендован", variant: "default" },
  dependency: { label: "зависимость", variant: "secondary" },
  needs_attention: { label: "вне диапазона macOS", variant: "warning" },
  incompatible: { label: "несовместим", variant: "destructive" },
};

export function KextsStep({ kexts, onToggle, darwin }: Props) {
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

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    return kexts.filter(
      (k) =>
        k.name.toLowerCase().includes(q) ||
        k.description.toLowerCase().includes(q) ||
        k.category.toLowerCase().includes(q)
    );
  }, [kexts, query]);

  return (
    <div className="space-y-5">
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">
            Подобрано кекстов: {selected.length}
          </CardTitle>
          <CardDescription className="text-xs">
            Для целевой macOS (Darwin {darwin}). Обязательные кексы нельзя снять —
            без них система не загрузится. Порядок загрузки OpenCore проставит снапшот.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-1.5">
            {selected.map((k) => (
              <Badge key={k.name} variant={STATUS_LABELS[k.status]?.variant ?? "secondary"}>
                {k.name}
              </Badge>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="relative">
        <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
        <Input
          placeholder="Поиск по каталогу кекстов..."
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
                  {category}
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
              Ничего не найдено
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}

function KextRow({ kext, onToggle }: { kext: KextEntry; onToggle: (name: string) => void }) {
  const status = STATUS_LABELS[kext.status];
  const locked = kext.required || kext.status === "dependency";

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
          {status && (
            <Badge variant={status.variant} className="text-[10px]">
              {status.label}
            </Badge>
          )}
        </div>
        <div className="text-muted-foreground truncate text-xs">{kext.description}</div>
      </div>
      {kext.repo && (
        <button
          onClick={() => openUrl(`https://github.com/${kext.repo}`)}
          className="text-muted-foreground hover:text-foreground shrink-0"
          title={`Репозиторий ${kext.repo}`}
        >
          <ExternalLink className="size-4" />
        </button>
      )}
    </div>
  );
}
