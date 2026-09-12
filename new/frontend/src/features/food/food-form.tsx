import { Field as FormField, Form, setErrors, useForm } from "@formisch/react";
import { AnimatePresence, motion } from "framer-motion";
import * as v from "valibot";

import {
  type FoodDetail,
  type Nutrient,
  type useAddFoodMutation,
  useFoodQuery,
  useMealSuggestionQuery,
} from "../../api/food.ts";
import { createId } from "../../lib/id.ts";
import { useIsReducedMotion } from "../../lib/use-is-reduced-motion.ts";
import { Button } from "../../ui/button/button.tsx";
import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { Field } from "../../ui/input/field.tsx";
import { Input } from "../../ui/input/input.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { amountSchema } from "./amount-schema.ts";

const nutrientCodes = [
  "energy",
  "protein",
  "carbohydrate",
  "fat",
  "fibre",
] as const;

const schema = v.object({
  meal: v.optional(
    v.picklist(
      ["continue_previous", "breakfast", "lunch", "snack", "dinner"],
      "Choose a meal",
    ),
  ),
  amount: amountSchema,
});

export function SelectedFood(props: {
  id: string;
  date: string;
  onClose: () => void;
  onSaved: () => void;
  mutation: ReturnType<typeof useAddFoodMutation>;
}) {
  const food = useFoodQuery(props.id);

  if (food.isError) {
    return (
      <div className="mt-4 rounded-2xl bg-danger-surface">
        <div className="flex flex-wrap items-center justify-between gap-4 p-4">
          <p role="alert" className="font-medium text-danger-fg">
            Error loading food
          </p>
          <div className="ml-auto flex gap-2">
            <Button type="button" variant="ghost" onClick={props.onClose}>
              Cancel
            </Button>
            <Button
              type="button"
              variant="outline"
              disabled={food.isFetching}
              onClick={() => void food.refetch()}
            >
              {food.isFetching ? "Retrying…" : "Try again"}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="mt-4 rounded-2xl border border-gray-200 p-4">
      <div className="mb-4 min-h-[2lh]">
        {food.data ? (
          <h3 className="font-medium text-gray-950">
            {food.data.display_name}
          </h3>
        ) : (
          <div aria-hidden="true" className="motion-safe:animate-pulse">
            <div className="flex h-lh items-center">
              <div className="h-5 w-48 max-w-full rounded bg-gray-200" />
            </div>
            <div className="flex h-lh items-center">
              <div className="h-5 w-32 max-w-full rounded bg-gray-200" />
            </div>
          </div>
        )}
      </div>
      <FoodAmountForm
        food={food.data}
        loading={food.isPending}
        date={props.date}
        mutation={props.mutation}
        onSaved={props.onSaved}
        onCancel={props.onClose}
      />
    </div>
  );
}

function NutritionSkeleton() {
  return (
    <div
      role="status"
      aria-label="Loading food details"
      className="motion-safe:animate-pulse"
    >
      <span className="sr-only">Loading food details</span>
      <div
        aria-hidden="true"
        className="grid grid-cols-3 gap-x-3 gap-y-2 text-sm sm:grid-cols-5"
      >
        {nutrientCodes.map((code) => (
          <div key={code}>
            <div className="flex h-lh items-center">
              <div className="h-4 w-16 rounded bg-gray-200" />
            </div>
            <div className="flex h-lh items-center">
              <div className="h-4 w-12 rounded bg-gray-200" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function FoodAmountForm(props: {
  food: FoodDetail | undefined;
  loading: boolean;
  date: string;
  onSaved: () => void;
  onCancel: () => void;
  mutation: ReturnType<typeof useAddFoodMutation>;
}) {
  const mutation = props.mutation;
  const isReducedMotion = useIsReducedMotion();
  const suggestion = useMealSuggestionQuery(props.date);
  const form = useForm({ schema, initialInput: { amount: "" } });
  return (
    <Form
      of={form}
      onSubmit={async ({ amount, meal }) => {
        if (!props.food || mutation.isPending) return;
        const selectedMeal = meal ?? suggestion.data?.meal;
        if (!selectedMeal) {
          setErrors(form, { path: ["meal"], errors: ["Choose a meal"] });
          return;
        }
        if (document.activeElement instanceof HTMLElement)
          document.activeElement.blur();
        try {
          await mutation.mutateAsync({
            renderKey: createId(),
            meal: selectedMeal,
            meal_id:
              selectedMeal === "continue_previous"
                ? (suggestion.data?.previous_meal_id ?? null)
                : null,
            meal_name:
              selectedMeal === "continue_previous"
                ? (suggestion.data?.previous_meal_name ?? null)
                : {
                    breakfast: "Breakfast",
                    lunch: "Lunch",
                    snack: "Snack",
                    dinner: "Dinner",
                  }[selectedMeal],
            food_id: props.food.id,
            food_name: props.food.display_name,
            food_brand: props.food.brand,
            amount,
            unit: props.food.basis_unit,
            eaten_at: new Date().toISOString(),
            nutrients: props.food.nutrients.map((nutrient) => ({
              ...nutrient,
              value: (nutrient.value * amount) / 100,
            })),
          });
          props.onSaved();
        } catch {
          /* Mutation displays the error and retains the amount for retry. */
        }
      }}
      className="space-y-4"
    >
      {props.food?.brand && (
        <p className="text-sm text-gray-600">{props.food.brand}</p>
      )}
      <FormField of={form} path={["amount"]}>
        {(field) => (
          <>
            {props.loading ? (
              <NutritionSkeleton />
            ) : props.food ? (
              <Nutrition
                nutrients={props.food.nutrients}
                amount={previewAmount(field.input ?? "")}
                label={
                  field.input
                    ? "Nutrition for this amount"
                    : `Nutrition per 100 ${props.food.basis_unit}`
                }
              />
            ) : null}
            <Field
              label={
                props.food ? `Amount (${props.food.basis_unit})` : "Amount"
              }
              error={field.errors?.[0]}
            >
              <Input
                {...field.props}
                value={field.input ?? ""}
                error={!!field.errors}
                inputMode="decimal"
                autoComplete="off"
                autoFocus
              />
            </Field>
          </>
        )}
      </FormField>
      <FormField of={form} path={["meal"]}>
        {(field) => (
          <Field label="Meal" error={field.errors?.[0]}>
            <span className="relative">
              <select
                {...field.props}
                aria-label="Meal"
                value={field.input ?? suggestion.data?.meal ?? ""}
                aria-invalid={field.errors?.length ? true : undefined}
                className="h-10 w-full appearance-none rounded-xl border border-transparent bg-[var(--input-bg)] pr-10 pl-3 font-[inherit] text-gray-1000 outline-2 outline-transparent outline-offset-[-1px] hover:bg-[var(--input-bg-alt)] focus-visible:outline-[var(--input-ring-active)] aria-invalid:outline-[var(--input-invalid-ring)]"
              >
                <option value="" disabled>
                  Choose meal
                </option>
                <option
                  value="continue_previous"
                  disabled={!suggestion.data?.previous_meal_id}
                >
                  Continue {suggestion.data?.previous_meal_name ?? "meal"}
                </option>
                <option value="breakfast">Breakfast</option>
                <option value="lunch">Lunch</option>
                <option value="snack">Snack</option>
                <option value="dinner">Dinner</option>
              </select>
              <ChevronRightIcon className="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 rotate-90" />
            </span>
          </Field>
        )}
      </FormField>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <AnimatePresence initial={false}>
          {mutation.isError &&
            mutation.variables?.food_id === props.food?.id && (
              <motion.p
                role="alert"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: isReducedMotion ? 0 : 0.16 }}
                className="rounded-lg bg-danger-surface text-sm text-danger-fg"
              >
                <span className="block px-2 py-1">Error adding food</span>
              </motion.p>
            )}
        </AnimatePresence>
        <div className="ml-auto flex gap-2">
          <Button type="button" variant="ghost" onClick={props.onCancel}>
            Cancel
          </Button>
          <Button type="submit" disabled={!props.food}>
            {mutation.isPending ? "Adding…" : "Add food"}
          </Button>
        </div>
      </div>
    </Form>
  );
}

function previewAmount(input: string) {
  if (!input.trim()) return 100;
  const amount = Number(input.replace(",", "."));
  return Number.isFinite(amount) && amount >= 0 && amount <= 100_000
    ? amount
    : 0;
}

export function Nutrition(props: {
  nutrients: Nutrient[];
  amount?: number;
  label: string;
}) {
  const { f } = useI18n();
  return (
    <dl
      aria-label={props.label}
      className="grid grid-cols-3 gap-x-3 gap-y-2 text-sm sm:grid-cols-5"
    >
      {nutrientCodes.map((code) => {
        const nutrient = props.nutrients.find((item) => item.code === code);
        return (
          <div key={code}>
            <dt className="text-gray-600">
              {nutrient?.name ??
                {
                  energy: "Energy",
                  protein: "Protein",
                  carbohydrate: "Carbs",
                  fat: "Fat",
                  fibre: "Fibre",
                }[code]}
            </dt>
            <dd className="tabular-nums text-gray-950">
              {nutrient
                ? `${code === "energy" ? f.calories((nutrient.value * (props.amount ?? 100)) / 100) : f.nutrient((nutrient.value * (props.amount ?? 100)) / 100)} ${nutrient.unit}`
                : "Unknown"}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}
