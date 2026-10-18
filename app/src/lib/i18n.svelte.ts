// RU/EN: тексты в JSON-словарях, язык переключается без перезапуска.
import ru from "../locales/ru.json";
import en from "../locales/en.json";

const dicts: Record<string, Record<string, string>> = { ru, en };

export const i18n = $state({ lang: "ru" as "ru" | "en" });

export function t(key: string, params?: Record<string, string | number>): string {
  let s = dicts[i18n.lang]?.[key] ?? dicts.ru[key] ?? key;
  if (params) for (const [k, v] of Object.entries(params)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
}

export function has(key: string): boolean {
  return key in (dicts[i18n.lang] ?? {});
}
