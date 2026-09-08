import { Autocomplete } from "@base-ui/react/autocomplete";
import type { Ref } from "react";

import { useFoodSearchQuery, type Food } from "../../api/food.ts";
import { Input } from "../../ui/input/input.tsx";

export function FoodSearch({
  inputRef,
  ...props
}: {
  query: string;
  onQuery: (query: string) => void;
  onSelect: (food: Food) => void;
  inputRef: Ref<HTMLInputElement>;
}) {
  const query = props.query.trim();
  const results = useFoodSearchQuery(query);
  const loading = query.length > 0 && results.isFetching;
  const items = query ? (results.data ?? []) : [];
  const noResults =
    query.length > 0 && results.isSuccess && results.data.length === 0;

  return (
    <Autocomplete.Root<Food>
      items={items}
      filteredItems={items}
      value={props.query}
      autoHighlight="always"
      itemToStringValue={(food) => food.display_name}
      onValueChange={(value, details) => {
        if (
          details.reason === "input-change" ||
          details.reason === "input-clear"
        )
          props.onQuery(value);
      }}
    >
      <div className="relative">
        <Autocomplete.Input
          id="food-search"
          ref={inputRef}
          render={<Input />}
          aria-label="Search foods"
          placeholder="Search foods, brands, meals..."
          maxLength={200}
          aria-busy={loading}
          className="pr-11"
        />
        <span
          aria-hidden="true"
          data-loading={loading || undefined}
          className="pointer-events-none absolute inset-y-0 right-2 flex w-7 items-center justify-center opacity-0 transition-opacity delay-300 duration-200 ease-out data-loading:opacity-100 data-loading:delay-0 data-loading:duration-0 motion-reduce:transition-none"
        >
          <span className="size-4 animate-spin rounded-full border-2 border-gray-350 border-t-gray-900 motion-reduce:animate-none" />
        </span>
      </div>
      <Autocomplete.Portal>
        <Autocomplete.Positioner
          sideOffset={6}
          align="start"
          className="z-20 w-[var(--anchor-width)] max-w-[var(--available-width)]"
        >
          <Autocomplete.Popup className="overflow-hidden rounded-xl border border-[var(--popover-border)] bg-[var(--surface-popover)] text-gray-950 shadow-lg">
            <Autocomplete.Status className="text-sm text-gray-600">
              {loading ? (
                <span className="sr-only">Searching…</span>
              ) : results.isError ? (
                <p role="alert" className="p-3 text-danger-fg">
                  Could not search foods. Try again.
                </p>
              ) : null}
            </Autocomplete.Status>
            <Autocomplete.Empty className="text-sm text-gray-600">
              {noResults && <p className="p-3">No foods found.</p>}
              {!query && <p className="p-3">Type a food name to search.</p>}
            </Autocomplete.Empty>
            <Autocomplete.List
              aria-busy={loading}
              className="max-h-[min(20rem,var(--available-height))] overflow-y-auto overscroll-contain p-1 empty:p-0"
            >
              {(food: Food) => (
                <Autocomplete.Item
                  key={food.id}
                  value={food}
                  onClick={() => props.onSelect(food)}
                  className="cursor-default rounded-lg px-3 py-2.5 outline-none data-highlighted:bg-[var(--popover-item-selected)]"
                >
                  <span className="block leading-snug">
                    {food.display_name}
                  </span>
                  <span className="mt-0.5 block text-sm text-gray-600">
                    {[
                      food.brand,
                      food.source === "fineli"
                        ? "Fineli"
                        : food.source === "open_food_facts"
                          ? "Open Food Facts"
                          : "Personal food",
                    ]
                      .filter(Boolean)
                      .join(" · ")}
                  </span>
                </Autocomplete.Item>
              )}
            </Autocomplete.List>
          </Autocomplete.Popup>
        </Autocomplete.Positioner>
      </Autocomplete.Portal>
    </Autocomplete.Root>
  );
}
