import { useSearch } from "wouter";

import { useI18n } from "../i18n/use-i18n.tsx";
import { WaterSection } from "../water/water-section.tsx";
import { DayNavigation } from "./day-navigation.tsx";

export default function HomePage() {
  const search = useSearch();
  const { f } = useI18n();
  const today = f.dateKey(new Date());
  const requestedDate = new URLSearchParams(search).get("date");
  const date = requestedDate && isDate(requestedDate) ? requestedDate : today;

  return (
    <main className="mx-auto max-w-[var(--page-width)] px-4 py-8 sm:px-7">
      <DayNavigation date={date} today={today} />
      <WaterSection date={date} />
    </main>
  );
}

function isDate(value: string) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const date = new Date(`${value}T12:00:00Z`);
  return (
    !Number.isNaN(date.valueOf()) && date.toISOString().slice(0, 10) === value
  );
}
