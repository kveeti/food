import { useSearch } from "wouter";

import type { User } from "../../api/user.ts";
import { WaterSection } from "../water/water-section.tsx";
import { DayNavigation } from "./day-navigation.tsx";

export default function HomePage(props: { user: User }) {
  const search = useSearch();
  const locale = props.user.locale!;
  const timezone = props.user.timezone!;
  const today = dateInTimezone(new Date(), timezone);
  const requestedDate = new URLSearchParams(search).get("date");
  const date = requestedDate && isDate(requestedDate) ? requestedDate : today;

  return (
    <main className="mx-auto max-w-[var(--page-width)] px-4 py-8 sm:px-7">
      <DayNavigation date={date} today={today} locale={locale} />
      <WaterSection date={date} locale={locale} timezone={timezone} />
    </main>
  );
}

function dateInTimezone(date: Date, timezone: string) {
  const parts = new Intl.DateTimeFormat("en", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    timeZone: timezone,
  }).formatToParts(date);
  const value = Object.fromEntries(
    parts.map((part) => [part.type, part.value]),
  );
  return `${value.year}-${value.month}-${value.day}`;
}

function isDate(value: string) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const date = new Date(`${value}T12:00:00Z`);
  return (
    !Number.isNaN(date.valueOf()) && date.toISOString().slice(0, 10) === value
  );
}
