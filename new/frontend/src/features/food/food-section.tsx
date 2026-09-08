import {
  AnimatePresence,
  motion,
  useIsPresent,
  useReducedMotion,
} from "framer-motion";
import { useState } from "react";

import { type FoodMealView, useFoodMealsQuery } from "../../api/food.ts";
import { useI18n } from "../i18n/use-i18n.tsx";
import { FoodEntryRow } from "./food-entry-row.tsx";

export function FoodSection(props: { date: string }) {
  const meals = useFoodMealsQuery(props.date);

  return (
    <section aria-labelledby="food-heading" className="mt-8">
      <h2 id="food-heading" className="mb-4 text-lg font-medium text-gray-950">
        Food
      </h2>
      {meals.isError && (
        <p role="alert" className="mt-4 text-danger-fg">
          Could not load food entries.
        </p>
      )}
      {!meals.isPending && meals.meals.length === 0 && (
        <p className="mt-5 text-gray-600">No food logged for this day.</p>
      )}
      {(meals.data || meals.meals.length > 0) && (
        <FoodMeals key={props.date} date={props.date} meals={meals.meals} />
      )}
    </section>
  );
}

function FoodMeals(props: { date: string; meals: FoodMealView[] }) {
  const [initialMeals] = useState(props.meals);

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
              first={index === 0}
              animateEntries={!initialMeals.includes(meal)}
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
}) {
  const { f } = useI18n();
  const reducedMotion = useReducedMotion();
  const present = useIsPresent();
  const energy = props.meal.entries.flatMap((entry) =>
    entry.nutrients.filter((nutrient) => nutrient.code === "energy"),
  );

  return (
    <motion.article
      aria-hidden={!present || undefined}
      inert={!present}
      layout={reducedMotion ? false : "position"}
      className="flow-root"
      initial={{ opacity: 0 }}
      animate={{ height: "auto", opacity: 1 }}
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
              ? `${f.amount(energy.reduce((total, nutrient) => total + nutrient.value, 0))} kcal`
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
