import { Cpu, HardDriveDownload, Loader2, Sparkles } from "lucide-react";
import appIcon from "@/assets/openefi-icon.png";
import { useI18n } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface Props {
  scanning: boolean;
  onScan: () => void;
}

export function WelcomeStep({ scanning, onScan }: Props) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col items-center py-10">
      <img src={appIcon} alt="OpenEFI" className="mb-6 size-20" />
      <h1 className="mb-2 text-2xl font-bold">{t("welcome.title")}</h1>
      <p className="text-muted-foreground mb-8 max-w-lg text-center text-sm leading-relaxed">
        {t("welcome.desc")}
      </p>
      <Button size="lg" onClick={onScan} disabled={scanning} className="gap-2">
        {scanning ? (
          <>
            <Loader2 className="size-4 animate-spin" />
            {t("welcome.scanning")}
          </>
        ) : (
          <>
            <HardDriveDownload className="size-4" />
            {t("welcome.scan")}
          </>
        )}
      </Button>

      <div className="mt-12 grid w-full grid-cols-1 gap-4 sm:grid-cols-3">
        {[
          {
            icon: Sparkles,
            title: t("welcome.f1.title"),
            text: t("welcome.f1.text"),
          },
          {
            icon: Cpu,
            title: t("welcome.f2.title"),
            text: t("welcome.f2.text"),
          },
          {
            icon: HardDriveDownload,
            title: t("welcome.f3.title"),
            text: t("welcome.f3.text"),
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
