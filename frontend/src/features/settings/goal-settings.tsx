import { Field as FormField, Form, setInput, useForm } from "@formisch/react";
import { useState } from "react";
import * as v from "valibot";

import {
  type Goals,
  type NutrientGoal,
  useGoalsQuery,
  useSaveGoalsMutation,
} from "../../api/goals.ts";
import { type SaveStatus, useAutosave } from "../../lib/use-autosave.ts";
import { DateInput } from "../../ui/input/date-input.tsx";
import { Field } from "../../ui/input/field.tsx";
import { Input } from "../../ui/input/input.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { GoalSettingsSkeleton } from "./goal-settings-skeleton.tsx";
import { AddNutrientGoalRow, NutrientGoalRow } from "./nutrient-goals.tsx";

const number = v.pipe(
  v.string(),
  v.trim(),
  v.nonEmpty("Enter a number"),
  v.transform((value) => Number(value.replace(",", "."))),
  v.number("Enter a number"),
  v.finite("Enter a finite number"),
);

const optionalPositive = v.union([
  v.literal(""),
  v.pipe(number, v.minValue(Number.MIN_VALUE, "Must be greater than 0")),
]);

const optionalWater = v.union([
  v.literal(""),
  v.pipe(
    number,
    v.minValue(10, "Must be at least 10"),
    v.maxValue(100_000, "Must be at most 100000"),
  ),
]);

const schema = v.pipe(
  v.object({
    dailyBurn: optionalPositive,
    adjustment: number,
    water: optionalWater,
    nutrients: v.array(
      v.object({
        code: v.string(),
        value: optionalPositive,
      }),
    ),
  }),
  v.forward(
    v.partialCheck(
      [["dailyBurn"], ["adjustment"]],
      (values) =>
        values.dailyBurn === "" || values.dailyBurn + values.adjustment > 0,
      "The calorie goal must be greater than 0",
    ),
    ["adjustment"],
  ),
);

export function GoalSettings(props: {
  onSaveStatusChange: (status: SaveStatus) => void;
}) {
  const { f } = useI18n();
  const [startsOn, setStartsOn] = useState(f.dateKey(new Date()));
  const goals = useGoalsQuery(startsOn);

  return (
    <section aria-labelledby="goals-heading" className="mt-12">
      <header className="mb-5">
        <h2 id="goals-heading" className="text-lg font-medium text-gray-950">
          Goals
        </h2>
        <p className="mt-1 text-sm text-gray-600">
          These goals apply from the chosen date until the next change.
        </p>
      </header>
      <Field label="Start date">
        <DateInput
          value={startsOn}
          onChange={(event) => setStartsOn(event.currentTarget.value)}
        />
      </Field>
      {goals.isPending && <GoalSettingsSkeleton />}
      {goals.isError && (
        <p role="alert" className="mt-4 text-danger-fg">
          Error loading goals.
        </p>
      )}
      {goals.data && (
        <GoalForm
          key={startsOn}
          startsOn={startsOn}
          goals={goals.data}
          onSaveStatusChange={props.onSaveStatusChange}
        />
      )}
    </section>
  );
}

function GoalForm(props: {
  startsOn: string;
  goals: Goals;
  onSaveStatusChange: (status: SaveStatus) => void;
}) {
  const { f } = useI18n();
  const mutation = useSaveGoalsMutation();
  const [shownCodes, setShownCodes] = useState(() =>
    props.goals.nutrients
      .filter((nutrient) => nutrient.show_by_default || nutrient.goal !== null)
      .map((nutrient) => nutrient.code),
  );
  const form = useForm({
    schema,
    initialInput: {
      dailyBurn: inputValue(props.goals.daily_burn_kcal),
      adjustment: inputValue(props.goals.food_adjustment_kcal ?? 0),
      water: inputValue(props.goals.water_ml),
      nutrients: props.goals.nutrients.map((nutrient) => ({
        code: nutrient.code,
        value: inputValue(nutrient.goal),
      })),
    },
  });
  const shown = shownCodes
    .map((code) => props.goals.nutrients.find((item) => item.code === code))
    .filter((nutrient): nutrient is NutrientGoal => !!nutrient);
  const available = props.goals.nutrients.filter(
    (nutrient) => !shownCodes.includes(nutrient.code),
  );
  const autosave = useAutosave({
    form,
    save: async (values) => {
      const burn = optionalValue(values.dailyBurn);
      const adjustment = burn === null ? null : values.adjustment;
      await mutation.mutateAsync({
        starts_on: props.startsOn,
        daily_burn_kcal: burn,
        food_adjustment_kcal: adjustment,
        water_ml: optionalValue(values.water),
        nutrients: values.nutrients.flatMap((nutrient) =>
          nutrient.value === ""
            ? []
            : [{ code: nutrient.code, value: nutrient.value }],
        ),
      });
    },
    onStatusChange: props.onSaveStatusChange,
  });

  return (
    <Form
      of={form}
      className="mt-5 min-w-0 space-y-6"
      onChange={(event) => {
        if (event.target instanceof HTMLInputElement) autosave.schedule();
      }}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) {
          autosave.flush();
        }
      }}
      onSubmit={() => autosave.flush()}
    >
      <div>
        <h3 className="mb-3 font-medium text-gray-950">Calories</h3>
        <div className="grid gap-4 sm:grid-cols-2">
          <FormField of={form} path={["dailyBurn"]}>
            {(field) => (
              <Field label="Daily burn (kcal)" error={field.errors?.[0]}>
                <Input
                  {...field.props}
                  value={field.input ?? ""}
                  error={!!field.errors}
                  inputMode="decimal"
                  autoComplete="off"
                />
              </Field>
            )}
          </FormField>
          <FormField of={form} path={["adjustment"]}>
            {(field) => (
              <Field
                label="Deficit or surplus (kcal)"
                error={field.errors?.[0]}
              >
                <Input
                  {...field.props}
                  value={field.input ?? ""}
                  error={!!field.errors}
                  inputMode="decimal"
                  autoComplete="off"
                />
                <span className="text-sm text-gray-600">
                  Use a minus for a deficit.
                </span>
              </Field>
            )}
          </FormField>
        </div>
        <FormField of={form} path={["dailyBurn"]}>
          {(burn) => (
            <FormField of={form} path={["adjustment"]}>
              {(adjustment) => {
                const burnValue = previewNumber(burn.input);
                const adjustmentValue = previewNumber(adjustment.input);
                const goal =
                  burnValue === null || adjustmentValue === null
                    ? null
                    : burnValue + adjustmentValue;
                return (
                  <p className="mt-3 text-sm text-gray-700">
                    Food goal:{" "}
                    {goal !== null && goal > 0 ? f.calories(goal) : "—"} kcal
                  </p>
                );
              }}
            </FormField>
          )}
        </FormField>
      </div>

      <FormField of={form} path={["water"]}>
        {(field) => (
          <Field label="Water (ml)" error={field.errors?.[0]}>
            <Input
              {...field.props}
              value={field.input ?? ""}
              error={!!field.errors}
              inputMode="numeric"
              autoComplete="off"
            />
          </Field>
        )}
      </FormField>

      <div>
        <h3 className="mb-3 font-medium text-gray-950">Nutrients</h3>
        <div className="space-y-3">
          {shown.map((nutrient, rowIndex) => {
            const nutrientIndex = props.goals.nutrients.findIndex(
              (item) => item.code === nutrient.code,
            );
            return (
              <FormField
                key={nutrient.code}
                of={form}
                path={["nutrients", nutrientIndex, "value"]}
              >
                {(field) => (
                  <NutrientGoalRow
                    nutrient={nutrient}
                    options={props.goals.nutrients.filter(
                      (item) =>
                        item.code === nutrient.code ||
                        !shownCodes.includes(item.code),
                    )}
                    input={field.input ?? ""}
                    inputProps={field.props}
                    error={field.errors?.[0]}
                    onNutrientChange={(code) => {
                      if (!code || code === nutrient.code) return;
                      const nextIndex = props.goals.nutrients.findIndex(
                        (item) => item.code === code,
                      );
                      setInput(form, {
                        path: ["nutrients", nutrientIndex, "value"],
                        input: "",
                      });
                      setInput(form, {
                        path: ["nutrients", nextIndex, "value"],
                        input: "",
                      });
                      setShownCodes((codes) =>
                        codes.map((shownCode, index) =>
                          index === rowIndex ? code : shownCode,
                        ),
                      );
                      autosave.schedule();
                    }}
                  />
                )}
              </FormField>
            );
          })}
        </div>
        {available.length > 0 && (
          <AddNutrientGoalRow
            nutrients={available}
            onSelect={(code) => {
              if (!code) return;
              setShownCodes((codes) => [...codes, code]);
            }}
          />
        )}
      </div>

      {mutation.isError && (
        <p role="alert" className="text-danger-fg">
          Error saving goals
        </p>
      )}
    </Form>
  );
}

function inputValue(value: number | null) {
  return value === null ? "" : String(Math.round(value * 100) / 100);
}

function optionalValue(value: "" | number) {
  return value === "" ? null : value;
}

function previewNumber(value: string | undefined) {
  if (!value?.trim()) return null;
  const number = Number(value.replace(",", "."));
  return Number.isFinite(number) ? number : null;
}
