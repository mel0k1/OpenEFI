import { Cpu, HardDriveDownload, Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  scanning: boolean;
  onScan: () => void;
}

export function WelcomeStep({ scanning, onScan }: Props) {
  return (
    <div className="flex flex-col items-center py-10">
      <div className="mb-6 flex size-20 items-center justify-center rounded-2xl bg-primary text-primary-foreground shadow-lg">
        <Cpu className="size-10" />
      </div>
      <h1 className="mb-2 text-2xl font-bold">Добро пожаловать в OpenEFI</h1>
      <p className="text-muted-foreground mb-8 max-w-lg text-center text-sm leading-relaxed">
        Определите железо компьютера, получите подборку совместимых кекстов,
        рекомендацию SMBIOS и сгенерируйте серийные номера для вашего хакинтоша.
        Логика подбора основана на опыте OpCore-Simplify и RapidEFI.
      </p>
      <Button size="lg" onClick={onScan} disabled={scanning} className="gap-2">
        {scanning ? (
          <>
            <Loader2 className="size-4 animate-spin" />
            Сканирую железо...
          </>
        ) : (
          <>
            <HardDriveDownload className="size-4" />
            Сканировать железо
          </>
        )}
      </Button>

      <div className="mt-12 grid w-full grid-cols-1 gap-4 sm:grid-cols-3">
        {[
          {
            icon: Sparkles,
            title: "Автоподбор кекстов",
            text: "Lilu, VirtualSMC, WhateverGreen, AppleALC и сетевые драйверы — по вашим PCI ID",
          },
          {
            icon: Cpu,
            title: "SMBIOS и серийники",
            text: "Рекомендация модели Mac по CPU и платформе, генерация Serial, MLB, ROM и UUID",
          },
          {
            icon: HardDriveDownload,
            title: "Совместимость macOS",
            text: "Оценка максимальной версии macOS по GPU и поколению процессора",
          },
        ].map((f) => (
          <Card key={f.title}>
            <CardHeader>
              <f.icon className="text-muted-foreground mb-1 size-5" />
              <CardTitle className="text-sm">{f.title}</CardTitle>
              <CardDescription className="text-xs leading-relaxed">{f.text}</CardDescription>
            </CardHeader>
            <CardContent />
          </Card>
        ))}
      </div>
    </div>
  );
}
