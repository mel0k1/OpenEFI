import { useCallback, useState } from "react";

export function useCopy(timeout = 1500) {
  const [copied, setCopied] = useState<string | null>(null);

  const copy = useCallback(
    async (text: string, key: string) => {
      try {
        await navigator.clipboard.writeText(text);
      } catch {
        const ta = document.createElement("textarea");
        ta.value = text;
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        document.body.removeChild(ta);
      }
      setCopied(key);
      setTimeout(() => setCopied(null), timeout);
    },
    [timeout]
  );

  return { copied, copy };
}
