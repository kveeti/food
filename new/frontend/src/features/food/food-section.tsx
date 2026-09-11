import { useMutationState } from "@tanstack/react-query";
import {
  AnimatePresence,
  motion,
  useIsPresent,
  useReducedMotion,
} from "framer-motion";
import { useState } from "react";

import { type FoodMealView, useFoodMealsQuery } from "../../api/food.ts";
import { Button } from "../../ui/button/button.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { FoodEntryRow } from "./food-entry-row.tsx";

export function FoodSection(props: { date: string }) {
  const meals = useFoodMealsQuery(props.date);
  const deletingEntryIds = new Set(
    useMutationState({
      filters: {
        mutationKey: ["food-entry", props.date, "delete"],
        status: "pending",
      },
      select: (mutation) => mutation.state.variables as string,
    }),
  );
  const deletingMealIds = new Set(
    meals.meals
      .filter(
        (meal) =>
          meal.entries.length > 0 &&
          meal.entries.every(
            (entry) => entry.id !== null && deletingEntryIds.has(entry.id),
          ),
      )
      .map((meal) => meal.renderKey),
  );

  return (
    <section aria-labelledby="food-heading" className="mt-8">
      <h2 id="food-heading" className="mb-4 text-lg font-medium text-gray-950">
        Food
      </h2>
      {meals.isPending ? (
        <FoodMealsSkeleton />
      ) : meals.isError ? (
        <FoodMealsError
          retrying={meals.isFetching}
          onRetry={() => void meals.refetch()}
        />
      ) : (
        <div
          aria-busy={meals.isPlaceholderData}
          inert={meals.isPlaceholderData}
          className={`transition-[filter,opacity] duration-200 ${meals.isPlaceholderData ? "pointer-events-none opacity-70 blur-[1px]" : ""}`}
        >
          <FoodMeals
            key={`${props.date}:${meals.isPlaceholderData ? "stale" : "current"}`}
            date={props.date}
            meals={meals.meals}
            deletingMealIds={deletingMealIds}
          />
          {deletingMealIds.size === meals.meals.length && <FoodEmptyState />}
        </div>
      )}
    </section>
  );
}

function FoodMealsError(props: { retrying: boolean; onRetry: () => void }) {
  return (
    <div className="rounded-2xl bg-danger-surface">
      <div className="flex items-center justify-between gap-4 p-4">
        <p role="alert" className="font-medium text-danger-fg">
          Error loading meals
        </p>
        <Button
          type="button"
          variant="outline"
          disabled={props.retrying}
          onClick={props.onRetry}
        >
          {props.retrying ? "Retrying…" : "Try again"}
        </Button>
      </div>
    </div>
  );
}

function FoodEmptyState() {
  return (
    <div className="rounded-2xl bg-gray-100">
      <div className="flex items-center gap-3 p-4">
        <div className="grid size-10 shrink-0 place-items-center rounded-full bg-gray-200 text-gray-600">
          <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="size-5"
          >
            <path d="M6 3v6a3 3 0 0 0 6 0V3M9 3v18M16 3v18M16 3c2.8 1.2 4 4.2 4 8h-4" />
          </svg>
        </div>
        <div>
          <p className="font-medium text-gray-950">
            No meals logged for this day
          </p>
          <p className="mt-0.5 text-sm text-gray-600">
            Search above to log food
          </p>
        </div>
      </div>
    </div>
  );
}

function FoodMealsSkeleton() {
  return (
    <div
      role="status"
      aria-label="Loading food entries"
      className="motion-safe:animate-pulse"
    >
      <div aria-hidden="true">
        <div className="rounded-xl bg-gray-100">
          <div className="flex min-h-12 items-center justify-between gap-4 px-3 py-2">
            <div className="h-5 w-24 rounded bg-gray-200" />
            <div className="h-4 w-28 rounded bg-gray-200" />
          </div>
        </div>
        <div className="flex min-h-14 items-center justify-between gap-4 border-b border-gray-200 px-2">
          <div className="h-5 w-36 rounded bg-gray-200" />
          <div className="h-4 w-20 rounded bg-gray-200" />
        </div>
        <div className="flex min-h-14 items-center justify-between gap-4 border-b border-gray-200 px-2">
          <div className="h-5 w-28 rounded bg-gray-200" />
          <div className="h-4 w-20 rounded bg-gray-200" />
        </div>
      </div>
    </div>
  );
}

function FoodMeals(props: {
  date: string;
  meals: FoodMealView[];
  deletingMealIds: ReadonlySet<string>;
}) {
  const [initialMeals] = useState(props.meals);
  const firstVisibleMeal = props.meals.find(
    (meal) => !props.deletingMealIds.has(meal.renderKey),
  );

  return (
    <div>
      <AnimatePresence initial={false}>
        {props.meals.map((meal, index) => {
          return (
            <FoodMeal
              key={meal.renderKey}
              id={meal.renderKey}
              meal={meal}
              date={props.date}
              first={index === 0 || meal === firstVisibleMeal}
              animateEntries={!initialMeals.includes(meal)}
              deleting={props.deletingMealIds.has(meal.renderKey)}
            />
          );
        })}
      </AnimatePresence>
    </div>
  );
}

function FoodMeal(props: {
  id: string;
  meal: FoodMealView;
  date: string;
  first: boolean;
  animateEntries: boolean;
  deleting: boolean;
}) {
  const { f } = useI18n();
  const reducedMotion = useReducedMotion();
  const present = useIsPresent();
  const energy = props.meal.entries.flatMap((entry) =>
    entry.nutrients.filter((nutrient) => nutrient.code === "energy"),
  );

  return (
    <motion.article
      aria-hidden={props.deleting || !present || undefined}
      inert={props.deleting || !present}
      layout={reducedMotion ? false : "position"}
      className="flow-root"
      initial={{ opacity: 0 }}
      animate={{
        height: props.deleting ? 0 : "auto",
        opacity: props.deleting ? 0 : 1,
      }}
      exit={{ height: 0, opacity: 0 }}
      transition={{
        duration: reducedMotion ? 0 : 0.25,
        ease: [0.16, 1, 0.3, 1],
      }}
      aria-labelledby={`meal-${props.id}`}
    >
      <div className={props.first ? undefined : "mt-6"}>
        <header className="-mx-3 flex min-h-12 items-center justify-between gap-4 rounded-xl bg-gray-100 px-3 py-2">
          <h3 id={`meal-${props.id}`} className="font-medium text-gray-950">
            {props.meal.name ?? "Meal"}
          </h3>
          <p className="shrink-0 text-right text-sm tabular-nums text-gray-600">
            <time dateTime={props.meal.started_at}>
              {f.time(new Date(props.meal.started_at))}
            </time>
            {" · "}
            {energy.length
              ? `${f.calories(energy.reduce((total, nutrient) => total + nutrient.value, 0))} kcal`
              : "Energy unknown"}
          </p>
        </header>
        <ul aria-label={`${props.meal.name ?? "Meal"} foods`}>
          <AnimatePresence initial={props.animateEntries}>
            {props.meal.entries.map((entry) => (
              <FoodEntryRow
                key={entry.renderKey}
                entry={entry}
                date={props.date}
              />
            ))}
          </AnimatePresence>
        </ul>
      </div>
    </motion.article>
  );
}
