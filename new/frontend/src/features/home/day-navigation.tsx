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
    <header className="grid grid-cols-[2.5rem_1fr_2.5rem] items-center gap-2">
      <DayLink
        href={dayUrl(moveDate(props.date, -1), props.today)}
        label="Previous day"
      >
        ‹
      </DayLink>
      <div className="text-center">
        <p className="text-sm text-gray-600">{eyebrow}</p>
        <h1 className="text-xl font-semibold tracking-tight text-gray-950">
          <time dateTime={props.date}>{title}</time>
        </h1>
      </div>
      <DayLink
        href={dayUrl(moveDate(props.date, 1), props.today)}
        label="Next day"
      >
        ›
      </DayLink>
    </header>
  );
}

function DayLink(props: { href: string; label: string; children: string }) {
  return (
    <ImmediateNavLink
      href={props.href}
      aria-label={props.label}
      className="grid size-10 place-items-center rounded-lg text-xl text-gray-700 outline-2 outline-transparent outline-offset-2 hover:bg-gray-150 focus-visible:outline-gray-500"
    >
      <span aria-hidden="true">{props.children}</span>
    </ImmediateNavLink>
  );
}
