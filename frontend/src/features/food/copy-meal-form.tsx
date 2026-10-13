import { Tabs } from "@base-ui/react/tabs";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";
import * as v from "valibot";
import { useLocation } from "wouter";

import {
  type CopyMealInput,
  type FoodMealView,
  type useCopyMealMutation,
  useFoodMealsQuery,
} from "../../api/food.ts";
import { Button } from "../../ui/button/button.tsx";
import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { DateInput } from "../../ui/input/date-input.tsx";
import { Field } from "../../ui/input/field.tsx";
import { Input } from "../../ui/input/input.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { amountSchema } from "./amount-schema.ts";

const mealNames = {
  breakfast: "Breakfast",
  lunch: "Lunch",
  snack: "Snack",
  dinner: "Dinner",
} as const;
type NewMealName = keyof typeof mealNames;

export function CopyMealForm(props: {
  meal: FoodMealView;
  selectedDate: string;
  onClose: () => void;
  mutation: ReturnType<typeof useCopyMealMutation>;
}) {
  const { f, timeZone } = useI18n();
  const [, navigate] = useLocation();
  const copy = props.mutation;
  const [date, setDate] = useState(() => f.dateKey(new Date()));
  const [mode, setMode] = useState("new");
  const [mealName, setMealName] = useState<NewMealName | "">(() => {
    const name = props.meal.name?.toLowerCase() ?? "";
    return name in mealNames ? (name as NewMealName) : "";
  });
  const [time, setTime] = useState(() =>
    new Intl.DateTimeFormat("en-GB", {
      timeZone,
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    }).format(new Date()),
  );
  const [existingChoice, setExistingChoice] = useState("continue_previous");
  const [foods, setFoods] = useState(() =>
    props.meal.entries.map((entry) => ({
      entry,
      isSelected: true,
      amount: String(entry.amount),
    })),
  );
  const isValidDate =
    /^\d{4}-\d{2}-\d{2}$/.test(date) &&
    !Number.isNaN(Date.parse(`${date}T12:00:00Z`)) &&
    new Date(`${date}T12:00:00Z`).toISOString().slice(0, 10) === date;
  const targetMeals = useFoodMealsQuery(date, isValidDate);
  const meals =
    !targetMeals.isPlaceholderData && !targetMeals.isError
      ? (targetMeals.data ?? []).filter(
          (meal) => meal.id && f.dateKey(new Date(meal.started_at)) === date,
        )
      : [];
  const latestMealId = meals
    .flatMap((meal) => meal.entries)
    .toSorted(
      (a, b) =>
        b.eaten_at.localeCompare(a.eaten_at) || b.id.localeCompare(a.id),
    )[0]?.meal_id;
  const latestMeal = meals.find((meal) => meal.id === latestMealId);
  const existingMeal = meals.find(
    (meal) =>
      meal.id ===
      (existingChoice === "continue_previous" ? latestMealId : existingChoice),
  );

  function submit() {
    if (copy.isPending || !props.meal.id || !isValidDate) return;
    const entries: CopyMealInput["entries"] = [];
    for (const food of foods.filter((food) => food.isSelected)) {
      const amount = v.safeParse(amountSchema, food.amount);
      if (!amount.success || !food.entry.id) return;
      entries.push({ entry_id: food.entry.id, amount: amount.output });
    }
    if (!entries.length) return;
    let target: CopyMealInput["target"];
    if (mode === "new") {
      if (!mealName || !/^([01]\d|2[0-3]):[0-5]\d$/.test(time)) return;
      target = { kind: "new", meal: mealName, time: `${time}:00` };
    } else {
      if (!existingMeal?.id) return;
      target = { kind: "existing", meal_id: existingMeal.id };
    }
    copy.mutate(
      { sourceMealId: props.meal.id, date, target, entries },
      {
        onSuccess: ({ meal_id }) => {
          props.onClose();
          if (date === props.selectedDate) return;
          toast.success("Meal copied", {
            description: f.dateOnly(date),
            action: {
              label: "View",
              onClick: () => navigate(`/?date=${date}&meal=${meal_id}`),
            },
          });
        },
      },
    );
  }

  return (
    <form
      noValidate
      className="space-y-5"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <Field label="Date">
        <DateInput
          value={date}
          onChange={(event) => {
            setDate(event.target.value);
            setExistingChoice("continue_previous");
            setMode("new");
          }}
        />
      </Field>
      <Tabs.Root value={mode} onValueChange={setMode}>
        <Tabs.List
          className="flex gap-1 rounded-xl bg-gray-100"
          aria-label="Copy destination"
        >
          <Tabs.Tab value="new" className={tabClassName}>
            New meal
          </Tabs.Tab>
          <Tabs.Tab
            value="existing"
            disabled={!meals.length}
            className={tabClassName}
          >
            Existing meal
          </Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel value="new">
          <div className="grid grid-cols-2 gap-3 pt-4">
            <Field label="Meal">
              <MealSelect
                value={mealName}
                onChange={(value) => setMealName(value as NewMealName)}
              >
                <option value="" disabled>
                  Choose meal
                </option>
                {Object.entries(mealNames).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </MealSelect>
            </Field>
            <Field label="Time">
              <Input
                type="time"
                value={time}
                onChange={(event) => setTime(event.target.value)}
              />
            </Field>
          </div>
        </Tabs.Panel>
        <Tabs.Panel value="existing">
          <div className="pt-4">
            <Field label="Meal">
              <MealSelect value={existingChoice} onChange={setExistingChoice}>
                <option value="continue_previous">
                  Continue last
                  {latestMeal
                    ? ` · ${latestMeal.name ?? "Meal"} · ${f.time(new Date(latestMeal.started_at))}`
                    : ""}
                </option>
                {meals
                  .filter((meal) => meal.id !== latestMealId)
                  .map((meal) => (
                    <option key={meal.id} value={meal.id!}>
                      {meal.name ?? "Meal"} ·{" "}
                      {f.time(new Date(meal.started_at))}
                    </option>
                  ))}
              </MealSelect>
            </Field>
          </div>
        </Tabs.Panel>
      </Tabs.Root>
      {isValidDate && targetMeals.isError && (
        <div className="flex items-center justify-between gap-3 text-sm">
          <p role="alert" className="text-danger-fg">
            Error loading existing meals
          </p>
          <Button
            type="button"
            variant="ghost"
            onClick={() => void targetMeals.refetch()}
          >
            Try again
          </Button>
        </div>
      )}
      <fieldset>
        <legend className="font-medium text-gray-950">Foods</legend>
        <ul className="mt-1 divide-y divide-gray-200">
          {foods.map((food, index) => (
            <li key={food.entry.renderKey}>
              <div className="flex items-center gap-3 py-3">
                <label className="flex min-w-0 flex-1 items-center gap-3">
                  <input
                    type="checkbox"
                    checked={food.isSelected}
                    onChange={(event) =>
                      setFoods((current) =>
                        current.map((item, itemIndex) =>
                          itemIndex === index
                            ? { ...item, isSelected: event.target.checked }
                            : item,
                        ),
                      )
                    }
                    className="size-5 shrink-0 accent-success-solid outline-offset-2"
                  />
                  <span
                    className={
                      food.isSelected ? "text-gray-950" : "text-gray-600"
                    }
                  >
                    <span className="block">{food.entry.food_name}</span>
                    {food.entry.food_brand && (
                      <span className="block text-sm text-gray-600">
                        {food.entry.food_brand}
                      </span>
                    )}
                  </span>
                </label>
                <div className="flex w-28 shrink-0 items-center gap-2">
                  <Input
                    aria-label={`${food.entry.food_name} amount (${food.entry.unit})`}
                    value={food.amount}
                    inputMode="decimal"
                    autoComplete="off"
                    disabled={!food.isSelected}
                    onChange={(event) =>
                      setFoods((current) =>
                        current.map((item, itemIndex) =>
                          itemIndex === index
                            ? { ...item, amount: event.target.value }
                            : item,
                        ),
                      )
                    }
                  />
                  <span className="text-sm text-gray-600">
                    {food.entry.unit}
                  </span>
                </div>
              </div>
            </li>
          ))}
        </ul>
      </fieldset>
      {copy.isError && (
        <p role="alert" className="text-sm text-danger-fg">
          Could not copy meal. Check the selected meal and try again.
        </p>
      )}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={props.onClose}>
          Cancel
        </Button>
        <Button type="submit">Copy meal</Button>
      </div>
    </form>
  );
}

const tabClassName =
  "h-10 flex-1 cursor-pointer rounded-xl px-3 text-sm font-medium text-gray-600 outline-2 outline-transparent outline-offset-[-2px] data-[active]:bg-gray-250 data-[active]:text-gray-950 focus-visible:outline-gray-500 disabled:cursor-default disabled:opacity-50";

function MealSelect(props: {
  value: string;
  onChange: (value: string) => void;
  children: ReactNode;
}) {
  return (
    <span className="relative">
      <select
        aria-label="Meal"
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
        className="h-10 w-full appearance-none rounded-xl border border-transparent bg-[var(--input-bg)] pr-10 pl-3 font-[inherit] text-gray-1000 outline-2 outline-transparent outline-offset-[-1px] hover:bg-[var(--input-bg-alt)] focus-visible:outline-[var(--input-ring-active)]"
      >
        {props.children}
      </select>
      <ChevronRightIcon className="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 rotate-90" />
    </span>
  );
}
