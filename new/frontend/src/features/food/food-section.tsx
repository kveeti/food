import { useMutationState } from "@tanstack/react-query";
import {
  AnimatePresence,
  motion,
  useIsPresent,
  useReducedMotion,
} from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { useSearch } from "wouter";

import {
  type FoodMealView,
  useDeleteMealMutation,
  useFoodMealsQuery,
} from "../../api/food.ts";
import { Button } from "../../ui/button/button.tsx";
import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { CopyMealDrawer } from "./copy-meal-drawer.tsx";
import { DeleteMealDialog } from "./delete-meal-dialog.tsx";
import { FoodEntryRow } from "./food-entry-row.tsx";

export function FoodSection(props: { date: string }) {
  const meals = useFoodMealsQuery(props.date);
  const search = useSearch();
  const requestedMealId = new URLSearchParams(search).get("meal");
  const viewedMealRef = useRef<string | null>(null);

  useEffect(() => {
    if (!requestedMealId) {
      viewedMealRef.current = null;
      return;
    }
    if (meals.isPlaceholderData || viewedMealRef.current === requestedMealId)
      return;
    const meal = document.getElementById(`food-meal-${requestedMealId}`);
    if (meal) {
      meal.scrollIntoView({ block: "start" });
      viewedMealRef.current = requestedMealId;
    }
  }, [requestedMealId, meals.isPlaceholderData, meals.data]);

  const deletingEntryIds = new Set(
    useMutationState({
      filters: {
        mutationKey: ["food-entry", props.date, "delete"],
        status: "pending",
      },
      select: (mutation) => mutation.state.variables as string,
    }),
  );
  const pendingMealIds = new Set(
    useMutationState({
      filters: {
        mutationKey: ["food-entry", props.date, "delete-meal"],
        status: "pending",
      },
      select: (mutation) => mutation.state.variables as string,
    }),
  );
  const deletingMealIds = new Set(
    meals.meals
      .filter(
        (meal) =>
          (meal.id !== null && pendingMealIds.has(meal.id)) ||
          (meal.entries.length > 0 &&
            meal.entries.every(
              (entry) => entry.id !== null && deletingEntryIds.has(entry.id),
            )),
      )
      .map((meal) => meal.renderKey),
  );

  return (
    <div>
      {meals.isPending ? (
        <FoodMealsSkeleton />
      ) : meals.isError ? (
        <div className="px-[var(--page-padding)]">
          <FoodMealsError
            retrying={meals.isFetching}
            onRetry={() => void meals.refetch()}
          />
        </div>
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
          {deletingMealIds.size === meals.meals.length && (
            <div className="px-[var(--page-padding)]">
              <FoodEmptyState />
            </div>
          )}
        </div>
      )}
    </div>
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
        <div className="bg-gray-100 sm:rounded-xl">
          <div className="flex min-h-12 items-center justify-between gap-4 px-[var(--page-padding)] py-2">
            <div className="h-5 w-24 rounded bg-gray-200" />
            <div className="h-4 w-28 rounded bg-gray-200" />
          </div>
        </div>
        <div className="px-[var(--page-padding)]">
          <div className="flex min-h-14 items-center justify-between gap-4 px-2">
            <div className="h-5 w-36 rounded bg-gray-200" />
            <div className="h-4 w-20 rounded bg-gray-200" />
          </div>
          <div className="flex min-h-14 items-center justify-between gap-4 border-b border-gray-200 px-2">
            <div className="h-5 w-28 rounded bg-gray-200" />
            <div className="h-4 w-20 rounded bg-gray-200" />
          </div>
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
  const [isExpanded, setIsExpanded] = useState(false);
  const deletion = useDeleteMealMutation(props.date);
  const isSaved =
    !!props.meal.id && props.meal.entries.every((entry) => entry.id);
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
      id={props.meal.id ? `food-meal-${props.meal.id}` : undefined}
      className="flow-root sm:scroll-mt-[calc(var(--desktop-day-navigation-height)+var(--desktop-search-height))]"
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
        <header className="sticky top-0 z-[5] bg-gray-100 sm:top-[calc(var(--desktop-day-navigation-height)+var(--desktop-search-height))] sm:rounded-[calc(var(--radius-xl)+0.5rem)]">
          <h3
            aria-labelledby={`meal-${props.id}`}
            className="rounded-[inherit]"
          >
            <button
              type="button"
              aria-label={`${isExpanded ? "Close" : "Open"} ${props.meal.name ?? "Meal"} meal`}
              aria-expanded={isExpanded}
              aria-controls={`meal-actions-${props.id}`}
              disabled={!isSaved}
              onClick={() => {
                if (isExpanded) deletion.reset();
                setIsExpanded(!isExpanded);
              }}
              className="block w-full rounded-[inherit] text-left font-[inherit] outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-150 focus-visible:outline-gray-500"
            >
              <span className="flex min-h-12 items-center gap-3 py-2 pr-[calc(var(--food-row-inset)+0.375rem)] pl-[var(--page-padding)]">
                <span
                  id={`meal-${props.id}`}
                  className="min-w-0 flex-1 font-medium text-gray-950"
                >
                  {props.meal.name ?? "Meal"}
                </span>
                <span className="shrink-0 text-right text-sm font-normal tabular-nums text-gray-600">
                  <time dateTime={props.meal.started_at}>
                    {f.time(new Date(props.meal.started_at))}
                  </time>
                  {" · "}
                  {energy.length
                    ? `${f.calories(energy.reduce((total, nutrient) => total + nutrient.value, 0))} kcal`
                    : "Energy unknown"}
                </span>
                <span className="grid size-6 shrink-0 place-items-center text-gray-600">
                  <ChevronRightIcon
                    className={`transition-transform duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none ${isExpanded ? "rotate-90" : ""}`}
                  />
                </span>
              </span>
            </button>
          </h3>
          <AnimatePresence initial={false}>
            {isExpanded && isSaved && (
              <MealActions
                id={`meal-actions-${props.id}`}
                date={props.date}
                meal={props.meal}
                deletion={deletion}
              />
            )}
          </AnimatePresence>
        </header>
        <ul aria-label={`${props.meal.name ?? "Meal"} foods`}>
          <AnimatePresence initial={props.animateEntries}>
            {props.meal.entries.map((entry, index) => (
              <FoodEntryRow
                key={entry.renderKey}
                entry={entry}
                date={props.date}
                last={index === props.meal.entries.length - 1}
              />
            ))}
          </AnimatePresence>
        </ul>
      </div>
    </motion.article>
  );
}

function MealActions(props: {
  id: string;
  date: string;
  meal: FoodMealView;
  deletion: ReturnType<typeof useDeleteMealMutation>;
}) {
  const isPresent = useIsPresent();
  const isReducedMotion = useReducedMotion();

  return (
    <motion.div
      id={props.id}
      inert={!isPresent}
      aria-hidden={!isPresent || undefined}
      initial={{ height: 0, opacity: 0 }}
      animate={{ height: "auto", opacity: 1 }}
      exit={{ height: 0, opacity: 0 }}
      transition={{
        height: {
          duration: isReducedMotion ? 0 : 0.22,
          ease: [0.16, 1, 0.3, 1],
        },
        opacity: { duration: isReducedMotion ? 0 : 0.14, ease: "easeOut" },
      }}
      className="overflow-hidden"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 p-2">
        <div className="flex flex-wrap items-center gap-2">
          <DeleteMealDialog
            mealName={props.meal.name ?? "Meal"}
            onConfirm={() => {
              if (props.deletion.isPending || !props.meal.id) return;
              props.deletion.mutate(props.meal.id);
            }}
          />
          {props.deletion.isError && (
            <p
              role="alert"
              className="rounded-lg bg-danger-surface text-sm text-danger-fg"
            >
              <span className="block px-2 py-1">Error deleting meal</span>
            </p>
          )}
        </div>
        <CopyMealDrawer meal={props.meal} selectedDate={props.date} />
      </div>
    </motion.div>
  );
}
