import type { Ref } from "react";
import { useLocation, useSearch } from "wouter";

import { FoodSearch } from "../food/food-search.tsx";

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
    <section aria-label="Search" className="mt-8">
      <FoodSearch
        query={params.get("q") ?? ""}
        onQuery={(q) => update({ q })}
        onSelect={(item) => update({ food: item.id, q: null })}
        inputRef={props.inputRef}
      />
    </section>
  );
}
