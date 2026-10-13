import { createContext, useContext } from "react";

export const I18nContext = createContext<ReturnType<
  typeof createI18nValue
> | null>(null);

export function useI18n() {
  const value = useContext(I18nContext);
  if (value === null) throw new Error("useI18n must be inside I18n");
  return value;
}

export function createI18nValue(locale: string, timeZone: string) {
  const number = new Intl.NumberFormat(locale);
  const amount = new Intl.NumberFormat(locale, { maximumFractionDigits: 2 });
  const nutrient = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  const calories = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
  const time = new Intl.DateTimeFormat(locale, {
    hour: "numeric",
    minute: "2-digit",
    timeZone,
  });
  // Calendar dates must not shift when the user's timezone changes.
  const dateOnly = new Intl.DateTimeFormat(locale, {
    dateStyle: "long",
    timeZone: "UTC",
  });
  const weekday = new Intl.DateTimeFormat(locale, {
    weekday: "long",
    timeZone: "UTC",
  });
  const dateKey = new Intl.DateTimeFormat("en", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    timeZone,
  });

  return {
    locale,
    timeZone,
    f: {
      number: number.format,
      amount: amount.format,
      nutrient: nutrient.format,
      calories: calories.format,
      time: time.format,
      dateOnly: (value: string) =>
        dateOnly.format(new Date(`${value}T12:00:00Z`)),
      weekday: (value: string) =>
        weekday.format(new Date(`${value}T12:00:00Z`)),
      dateKey: (value: Date) => {
        const parts = Object.fromEntries(
          dateKey.formatToParts(value).map((part) => [part.type, part.value]),
        );
        return `${parts.year}-${parts.month}-${parts.day}`;
      },
    },
  };
}
