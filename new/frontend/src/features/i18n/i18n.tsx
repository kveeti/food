import type { ReactNode } from "react";

import { createI18nValue, I18nContext } from "./use-i18n.tsx";

export function I18n(props: {
  locale: string;
  timeZone: string;
  children: ReactNode;
}) {
  const value = createI18nValue(props.locale, props.timeZone);
  return <I18nContext value={value}>{props.children}</I18nContext>;
}
