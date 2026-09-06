import type { ReactNode } from "react";

import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { ImmediateNavLink } from "../../ui/immediate-nav-link.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

function moveDate(value: string, days: number) {
  const date = new Date(`${value}T12:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

function dayUrl(date: string, today: string) {
  return date === today ? "/" : `/?date=${date}`;
}

export function DayNavigation(props: { date: string; today: string }) {
  const { f } = useI18n();
  const title = f.dateOnly(props.date);
  const eyebrow = props.date === props.today ? "Today" : f.weekday(props.date);

  return (
    <header className="fixed inset-x-0 bottom-[var(--nav-height)] z-10 border-t border-gray-200 bg-canvas/90 backdrop-blur-md sm:static sm:border-0 sm:bg-transparent sm:backdrop-blur-none">
      <div className="mx-auto flex h-20 max-w-[var(--page-width)] items-center justify-between gap-3 px-4 sm:h-auto sm:px-0">
        <div className="min-w-0">
          <p className="text-sm text-gray-600">{eyebrow}</p>
          <h1 className="text-xl font-semibold tracking-tight text-gray-950">
            <time dateTime={props.date}>{title}</time>
          </h1>
        </div>
        <div className="flex shrink-0 gap-2">
          <DayLink
            href={dayUrl(moveDate(props.date, -1), props.today)}
            label="Previous day"
          >
            <ChevronRightIcon className="rotate-180" />
          </DayLink>
          <DayLink
            href={dayUrl(moveDate(props.date, 1), props.today)}
            label="Next day"
          >
            <ChevronRightIcon />
          </DayLink>
        </div>
      </div>
    </header>
  );
}

function DayLink(props: { href: string; label: string; children: ReactNode }) {
  return (
    <ImmediateNavLink
      href={props.href}
      aria-label={props.label}
      className="grid size-10 place-items-center rounded-lg text-gray-700 outline-2 outline-transparent outline-offset-2 hover:bg-gray-150 focus-visible:outline-gray-500"
    >
      {props.children}
    </ImmediateNavLink>
  );
}
