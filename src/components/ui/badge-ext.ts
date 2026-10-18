// Re-export Badge + тип варианта (для таблиц соответствия вне JSX)
export { Badge, badgeVariants } from "@/components/ui/badge";

import type { VariantProps } from "class-variance-authority";
import { badgeVariants } from "@/components/ui/badge";

export type BadgeVariant = NonNullable<VariantProps<typeof badgeVariants>["variant"]>;
