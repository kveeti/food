import { useRef } from "react";
import { useSearch } from "wouter";

import { FoodSection } from "../food/food-section.tsx";
import { NewFoodSection } from "../food/new-food-section.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { SearchSection } from "../search/search-section.tsx";
import { WaterDrawer } from "../water/water-drawer.tsx";
import { DayNavigation } from "./day-navigation.tsx";
import { DaySummary } from "./day-summary.tsx";

export default function HomePage() {
  const search = useSearch();
  const searchInputRef = useRef<HTMLInputElement>(null);
  const { f } = useI18n();
  const today = f.dateKey(new Date());
  const requestedDate = new URLSearchParams(search).get("date");
  const date = requestedDate && isDate(requestedDate) ? requestedDate : today;

  return (
    <main className="min-h-0 w-full flex-1 overflow-hidden [--day-navigation-height:4rem] [--desktop-day-navigation-height:4.25rem] [--desktop-search-height:3.75rem] [--food-row-inset:0.25rem] [--summary-gap:2rem] [--summary-width:min(28rem,calc(100%_-_var(--summary-gap)))] sm:overflow-y-auto sm:overscroll-contain sm:[--food-row-inset:0.75rem] sm:[scrollbar-gutter:stable_both-edges]">
      <div className="mx-auto flex h-full min-h-0 w-full max-w-[var(--page-max-width)] flex-col sm:h-auto sm:min-h-full xl:grid xl:max-w-none xl:content-start xl:grid-cols-[minmax(0,1fr)_minmax(0,var(--page-max-width))_minmax(0,1fr)] xl:grid-rows-[auto_auto_auto]">
        <DayNavigation date={date} today={today} />
        <div className="z-10 shrink-0 flow-root bg-canvas/95 backdrop-blur-md sm:contents">
          <div className="px-[var(--page-padding)] pt-[env(safe-area-inset-top,0px)] sm:pt-0 xl:sticky xl:top-2 xl:col-start-1 xl:row-start-1 xl:row-span-3 xl:mr-[var(--summary-gap)] xl:w-[var(--summary-width)] xl:justify-self-end xl:self-start xl:p-0">
            <DaySummary date={date} />
          </div>
          <div className="px-[var(--page-padding)] sm:sticky sm:top-[var(--desktop-day-navigation-height)] sm:z-10 sm:flow-root sm:bg-canvas/95 sm:backdrop-blur-md xl:col-start-2 xl:row-start-2">
            <SearchSection inputRef={searchInputRef} />
          </div>
        </div>
        <section
          aria-label="Food"
          className="flow-root min-h-0 flex-1 overflow-y-auto overscroll-contain sm:overflow-visible xl:col-start-2 xl:row-start-3"
        >
          <div className="pb-[calc(var(--nav-clearance)+var(--day-navigation-height)+1rem)] sm:pb-[calc(var(--nav-clearance)+1rem)]">
            <div className="px-[var(--page-padding)]">
              <NewFoodSection
                date={date}
                onClose={() => searchInputRef.current?.focus()}
              />
            </div>
            <FoodSection date={date} />
          </div>
        </section>
      </div>
      <WaterDrawer date={date} />
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
