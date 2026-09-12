import type { ReactNode, UIEvent } from "react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useLocation } from "wouter";

import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

const dayRange = 365;

function moveDate(value: string, days: number) {
  const date = new Date(`${value}T12:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

function dayUrl(date: string, today: string) {
  return date === today ? "/" : `/?date=${date}`;
}

export function DayNavigation(props: { date: string; today: string }) {
  const { f, locale } = useI18n();
  const [, navigate] = useLocation();
  const scrollerRef = useRef<HTMLDivElement>(null);
  const frameRef = useRef<number>(undefined);
  const selectedDateRef = useRef(props.date);
  const changedByScrollRef = useRef(false);
  const scrollTargetRef = useRef<string>(undefined);
  const [days] = useState(() =>
    Array.from({ length: dayRange * 2 + 1 }, (_, index) =>
      moveDate(props.date, index - dayRange),
    ),
  );
  const weekday = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        weekday: "short",
        timeZone: "UTC",
      }),
    [locale],
  );
  const monthDay = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        month: "short",
        day: "numeric",
        timeZone: "UTC",
      }),
    [locale],
  );

  useLayoutEffect(() => {
    const index = days.indexOf(props.date);
    if (index < 0) return;

    selectedDateRef.current = props.date;
    if (changedByScrollRef.current) {
      changedByScrollRef.current = false;
      return;
    }
    const scroller = scrollerRef.current;
    const day = scroller?.querySelector<HTMLElement>(
      `[data-date="${props.date}"]`,
    );
    if (!scroller || !day) return;
    const scrollerBox = scroller.getBoundingClientRect();
    const dayBox = day.getBoundingClientRect();
    scroller.scrollTo({
      left:
        scroller.scrollLeft +
        dayBox.left -
        scrollerBox.left -
        (scroller.clientWidth - dayBox.width) / 2,
    });
  }, [days, props.date]);

  useEffect(
    () => () => {
      if (frameRef.current !== undefined) {
        cancelAnimationFrame(frameRef.current);
      }
    },
    [],
  );

  function handleScroll(event: UIEvent<HTMLDivElement>) {
    const scroller = event.currentTarget;
    if (frameRef.current !== undefined) {
      cancelAnimationFrame(frameRef.current);
    }
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = undefined;
      const firstDay = scroller.querySelector<HTMLElement>("[data-date]");
      const secondDay = firstDay?.nextElementSibling as HTMLElement | null;
      if (!firstDay || !secondDay) return;
      const scrollerBox = scroller.getBoundingClientRect();
      const firstDayBox = firstDay.getBoundingClientRect();
      const secondDayBox = secondDay.getBoundingClientRect();
      const dayStep = secondDayBox.left - firstDayBox.left;
      const firstDayCenter =
        scroller.scrollLeft +
        firstDayBox.left -
        scrollerBox.left +
        firstDayBox.width / 2;
      const index = Math.max(
        0,
        Math.min(
          days.length - 1,
          Math.round(
            (scroller.scrollLeft + scroller.clientWidth / 2 - firstDayCenter) /
              dayStep,
          ),
        ),
      );
      const date = days[index]!;
      if (scrollTargetRef.current !== undefined) {
        if (date === scrollTargetRef.current) {
          scrollTargetRef.current = undefined;
        }
        return;
      }
      if (date === selectedDateRef.current) return;

      selectedDateRef.current = date;
      changedByScrollRef.current = true;
      navigate(dayUrl(date, props.today), { replace: true });
    });
  }

  function scrollToDate(date: string) {
    scrollerRef.current
      ?.querySelector<HTMLElement>(`[data-date="${date}"]`)
      ?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
  }

  function selectDate(date: string) {
    if (date !== selectedDateRef.current) {
      selectedDateRef.current = date;
      changedByScrollRef.current = true;
      navigate(dayUrl(date, props.today), { replace: true });
    }
    scrollTargetRef.current = date;
    scrollToDate(date);
  }

  return (
    <header className="fixed inset-x-0 bottom-0 z-10 h-[calc(var(--day-navigation-height)+var(--nav-clearance))] border-t border-gray-200 bg-canvas/90 backdrop-blur-md sm:sticky sm:top-0 sm:bottom-auto sm:h-[var(--desktop-day-navigation-height)] sm:border-0 sm:bg-canvas/95 xl:col-start-2 xl:row-start-1">
      <div
        className="mx-auto grid h-[var(--day-navigation-height)] max-w-[var(--page-max-width)] grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 px-[var(--page-padding)] sm:h-[var(--desktop-day-navigation-height)]"
        data-day-navigation-content
      >
        <h1 className="sr-only">
          <time dateTime={props.date}>{f.dateOnly(props.date)}</time>
        </h1>
        <DayButton
          label="Previous day"
          onClick={() => selectDate(moveDate(props.date, -1))}
        >
          <ChevronRightIcon className="rotate-180" />
        </DayButton>
        <div
          ref={scrollerRef}
          aria-label="Choose day"
          className="grid min-w-0 flex-1 touch-pan-x snap-x snap-mandatory auto-cols-[4rem] grid-flow-col gap-2 overflow-x-auto overscroll-x-contain px-[calc(50%_-_2rem)] [mask-image:linear-gradient(to_right,transparent,black_12%,black_88%,transparent)] [-ms-overflow-style:none] [scroll-padding-inline:calc(50%_-_2rem)] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden [&::-webkit-scrollbar]:size-0"
          onPointerDown={() => {
            scrollTargetRef.current = undefined;
          }}
          onScroll={handleScroll}
          onWheel={() => {
            scrollTargetRef.current = undefined;
          }}
        >
          {days.map((date) => {
            const value = new Date(`${date}T12:00:00Z`);
            const selected = date === props.date;
            return (
              <button
                key={date}
                type="button"
                aria-current={selected ? "date" : undefined}
                aria-label={`${date === props.today ? "Today, " : ""}${f.dateOnly(date)}`}
                className="flex h-12 snap-center snap-normal flex-col items-center justify-center rounded-xl text-xs leading-[1.25] text-gray-600 opacity-[0.58] outline-2 outline-transparent outline-offset-[-2px] transition-[color,opacity,background-color] duration-[120ms] ease-out data-[selected=true]:bg-gray-150 data-[selected=true]:text-gray-950 data-[selected=true]:opacity-100 focus-visible:outline-gray-500"
                data-date={date}
                data-selected={selected || undefined}
                onClick={() => selectDate(date)}
              >
                <span>
                  {date === props.today ? "Today" : weekday.format(value)}
                </span>
                <span className="font-medium">{monthDay.format(value)}</span>
              </button>
            );
          })}
        </div>
        <DayButton
          label="Next day"
          onClick={() => selectDate(moveDate(props.date, 1))}
        >
          <ChevronRightIcon />
        </DayButton>
      </div>
    </header>
  );
}

function DayButton(props: {
  label: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={props.label}
      className="grid h-11 w-10 place-items-center rounded-lg text-gray-700 outline-2 outline-transparent outline-offset-2 hover:bg-gray-150 focus-visible:outline-gray-500 sm:h-10"
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
}
