import type { Ref } from "react";
import { useLocation, useSearch } from "wouter";

import { FoodSearch } from "../food/food-search.tsx";
import { WaterTrigger } from "../water/water-trigger.tsx";

export function SearchSection(props: { inputRef: Ref<HTMLInputElement> }) {
  const search = useSearch();
  const [, navigate] = useLocation();
  const params = new URLSearchParams(search);

  function update(changes: Record<string, string | null>) {
    const next = new URLSearchParams(window.location.search);
    for (const [key, value] of Object.entries(changes)) {
      if (value) next.set(key, value);
      else next.delete(key);
    }
    navigate(`/${next.size ? `?${next}` : ""}`, { replace: true });
  }

  return (
    <section
      aria-label="Search"
      className="mt-4 pb-3 sm:mt-0 sm:h-[var(--desktop-search-height)] sm:pt-2"
    >
      <div className="flex items-center gap-2">
        <div className="min-w-0 flex-1">
          <FoodSearch
            query={params.get("q") ?? ""}
            onQuery={(q) => update({ q })}
            onSelect={(item) => update({ food: item.id, q: null })}
            inputRef={props.inputRef}
          />
        </div>
        <WaterTrigger
          type="button"
          aria-label="Log water"
          className="grid size-10 shrink-0 cursor-pointer place-items-center rounded-xl bg-[var(--goal-water-soft)] text-gray-800 outline-2 outline-transparent outline-offset-2 hover:bg-[var(--goal-water-solid)] focus-visible:outline-gray-500"
        >
          <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            className="size-5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M12 3.5s-5 5.7-5 10a5 5 0 0 0 10 0c0-4.3-5-10-5-10Z" />
          </svg>
        </WaterTrigger>
      </div>
    </section>
  );
}
