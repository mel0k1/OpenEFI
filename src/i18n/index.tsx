import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ru, en } from "./ui";
import { BACKEND_EN, BACKEND_PATTERNS } from "./backend";

export type Lang = "ru" | "en";

const STORAGE_KEY = "openefi-lang";

export type TranslateParams = Record<string, string | number>;

interface I18nContextValue {
  lang: Lang;
  setLang: (l: Lang) => void;
  /** Перевод по семантическому ключу словаря */
  t: (key: string, params?: TranslateParams) => string;
  /** Перевод строки, пришедшей из бэкенда (русский источник) */
  tb: (s: string | null | undefined) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function initialLang(): Lang {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "en" || v === "ru") return v;
  } catch {
    // localStorage недоступен
  }
  return "ru";
}

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(initialLang);

  const setLang = useCallback((l: Lang) => {
    setLangState(l);
    try {
      localStorage.setItem(STORAGE_KEY, l);
    } catch {
      // localStorage недоступен
    }
  }, []);

  const t = useCallback(
    (key: string, params?: TranslateParams) => {
      let s = (lang === "en" ? en[key] : ru[key]) ?? ru[key] ?? key;
      if (params) {
        for (const [k, v] of Object.entries(params)) {
          s = s.split(`{${k}}`).join(String(v));
        }
      }
      return s;
    },
    [lang]
  );

  const tb = useCallback(
    (s: string | null | undefined) => {
      if (!s) return s ?? "";
      if (lang !== "en") return s;
      const direct = BACKEND_EN[s];
      if (direct) return direct;
      for (const [re, tmpl] of BACKEND_PATTERNS) {
        if (re.test(s)) return s.replace(re, tmpl);
      }
      return s;
    },
    [lang]
  );

  const value = useMemo(() => ({ lang, setLang, t, tb }), [lang, setLang, t, tb]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used inside <LangProvider>");
  return ctx;
}

/** Локаль для дат в зависимости от языка */
export function dateLocale(lang: Lang): string {
  return lang === "en" ? "en-US" : "ru-RU";
}
