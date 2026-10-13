import type { FieldElementProps } from "@formisch/react";
import type { ReactNode } from "react";

import type { NutrientGoal } from "../../api/goals.ts";
import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { Input } from "../../ui/input/input.tsx";

export function NutrientGoalRow(props: {
  nutrient: NutrientGoal;
  options: NutrientGoal[];
  input: string;
  inputProps: FieldElementProps;
  error?: string;
  onNutrientChange: (code: string) => void;
}) {
  const inputId = `nutrient-goal-${props.nutrient.code}`;

  return (
    <div className="flex items-start gap-2">
      <NutrientSelect
        label={`Nutrient for ${props.nutrient.name} goal`}
        value={props.nutrient.code}
        onChange={props.onNutrientChange}
      >
        {props.options.map((nutrient) => (
          <option key={nutrient.code} value={nutrient.code}>
            {nutrient.name} ({nutrient.unit})
          </option>
        ))}
      </NutrientSelect>
      <div className="w-28 shrink-0">
        <label htmlFor={inputId} className="sr-only">
          {props.nutrient.name} ({props.nutrient.unit})
        </label>
        <div className="relative">
          <Input
            {...props.inputProps}
            id={inputId}
            value={props.input}
            error={!!props.error}
            inputMode="decimal"
            autoComplete="off"
            className="pr-7 text-right tabular-nums"
            size="small"
          />
          <span
            aria-hidden="true"
            className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-sm text-gray-600"
          >
            {props.nutrient.unit}
          </span>
        </div>
        {props.error && (
          <span className="mt-1.5 block text-sm text-[var(--input-invalid-text)]">
            {props.error}
          </span>
        )}
      </div>
    </div>
  );
}

export function AddNutrientGoalRow(props: {
  nutrients: NutrientGoal[];
  onSelect: (code: string) => void;
}) {
  return (
    <div className="mt-3 flex">
      <NutrientSelect
        label="Add nutrient goal"
        value=""
        onChange={props.onSelect}
      >
        <option value="">Choose a nutrient</option>
        {props.nutrients.map((nutrient) => (
          <option key={nutrient.code} value={nutrient.code}>
            {nutrient.name} ({nutrient.unit})
          </option>
        ))}
      </NutrientSelect>
    </div>
  );
}

function NutrientSelect(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  children: ReactNode;
}) {
  return (
    <span className="relative min-w-0 flex-1">
      <select
        aria-label={props.label}
        value={props.value}
        onChange={(event) => props.onChange(event.currentTarget.value)}
        className="h-10 w-full appearance-none rounded-xl border border-transparent bg-[var(--input-bg)] pr-10 pl-3 font-[inherit] text-gray-1000 outline-2 outline-transparent outline-offset-[-1px] hover:bg-[var(--input-bg-alt)] focus-visible:outline-[var(--input-ring-active)]"
      >
        {props.children}
      </select>
      <ChevronRightIcon className="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 rotate-90" />
    </span>
  );
}
