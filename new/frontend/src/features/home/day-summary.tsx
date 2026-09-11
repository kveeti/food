import type { CSSProperties } from "react";

import {
  type GoalsWithProgress,
  useGoalsProgressQuery,
} from "../../api/goals.ts";
import { Button } from "../../ui/button/button.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

export function DaySummary(props: { date: string }) {
  const goals = useGoalsProgressQuery(props.date);
  const loading = goals.isPending || goals.isPlaceholderData;

  if (goals.isError) {
    return (
      <section aria-label="Day totals" className="mt-6">
        <div className="rounded-2xl bg-danger-surface">
          <div className="flex items-center justify-between gap-4 p-4">
            <p role="alert" className="font-medium text-danger-fg">
              Error loading daily totals
            </p>
            <Button
              type="button"
              variant="outline"
              disabled={goals.isFetching}
              onClick={() => void goals.refetch()}
            >
              {goals.isFetching ? "Retrying…" : "Try again"}
            </Button>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section aria-label="Day totals" aria-busy={loading} className="mt-6">
      <div
        className={`transition-[filter,opacity] duration-200 ${goals.isPlaceholderData ? "opacity-70 blur-[1px]" : ""}`}
      >
        <div className="grid grid-cols-2 gap-3">
          <GoalCard
            label="Calories"
            ariaLabel="Calorie total"
            value={goals.data?.progress.calories.eaten ?? 0}
            goal={goals.data?.calorie_goal_kcal ?? null}
            unit="kcal"
            tone="calorie"
            incomplete={goals.data?.progress.calories.incomplete}
            valueUnknown={goals.data?.progress.calories.unknown}
            initialLoading={goals.isPending}
          />
          <GoalCard
            label="Water"
            ariaLabel="Water total"
            value={goals.data?.progress.water_ml ?? 0}
            goal={goals.data?.water_ml ?? null}
            unit="ml"
            tone="water"
            initialLoading={goals.isPending}
          />
        </div>
        {goals.data ? (
          <NutrientTotals goals={goals.data} />
        ) : (
          <NutrientTotalsSkeleton />
        )}
      </div>
    </section>
  );
}

function GoalCard(props: {
  label: string;
  ariaLabel: string;
  value: number;
  goal: number | null;
  unit: string;
  tone: "calorie" | "water";
  incomplete?: boolean;
  valueUnknown?: boolean;
  initialLoading: boolean;
}) {
  const { f } = useI18n();
  const goal = props.goal;
  const hasGoal = goal !== null;
  const chartMax = Math.max(props.value, goal ?? 0, 1);
  const goalPercent = goal === null ? 0 : (goal / chartMax) * 100;
  const fillPercent =
    goal === null ? 0 : (Math.min(props.value, goal) / chartMax) * 100;
  const overflowPercent =
    goal !== null ? (Math.max(0, props.value - goal) / chartMax) * 100 : 0;
  const isOverGoal = overflowPercent > 0;
  const style = {
    "--fill-scale": fillPercent / 100,
    "--range-start": `${isOverGoal ? goalPercent : fillPercent}%`,
    "--range-size": `${isOverGoal ? overflowPercent : 100 - fillPercent}%`,
  } as CSSProperties;
  const unknown = props.valueUnknown && !props.initialLoading;
  const value = unknown
    ? "Unknown"
    : `${props.incomplete ? "At least " : ""}${props.tone === "calorie" ? f.calories(props.value) : f.amount(props.value)}`;
  const text =
    props.goal === null
      ? `${value}${unknown ? "" : ` ${props.unit}`}`
      : `${value}${unknown ? "" : ` / ${props.tone === "calorie" ? f.calories(props.goal) : f.amount(props.goal)} ${props.unit}`}`;

  return (
    <div
      className="goal-card relative overflow-hidden rounded-2xl bg-gray-150"
      data-tone={props.tone}
      data-has-goal={hasGoal || undefined}
      style={style}
    >
      {!props.initialLoading && !unknown && (
        <div aria-hidden="true" className="absolute inset-0">
          <div className="goal-card-fill absolute inset-y-0 left-0" />
          <div
            className="goal-card-range absolute inset-y-0"
            data-over-goal={isOverGoal || undefined}
          />
        </div>
      )}
      <div className="relative p-4">
        <p className="text-sm font-medium text-gray-700">{props.label}</p>
        {props.initialLoading ? (
          <div
            role="status"
            aria-label={`Loading ${props.label.toLowerCase()} total`}
            className="mt-1 flex h-lh items-center motion-safe:animate-pulse"
          >
            <div aria-hidden="true" className="h-5 w-32 rounded bg-gray-250" />
          </div>
        ) : (
          <p
            aria-label={props.ariaLabel}
            className="mt-1 font-medium tabular-nums text-gray-950"
          >
            {text}
          </p>
        )}
      </div>
    </div>
  );
}

function NutrientTotals(props: { goals: GoalsWithProgress }) {
  const { f } = useI18n();
  const progress = new Map(
    props.goals.progress.nutrients.map((nutrient) => [nutrient.code, nutrient]),
  );

  return (
    <dl className="mt-6 grid grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-4">
      {props.goals.nutrients.map((nutrient) => {
        const total = progress.get(nutrient.code);
        if (!total) return null;
        const amount = total.unknown
          ? "Unknown"
          : `${total.incomplete ? "At least " : ""}${f.nutrient(total.eaten)}`;
        return (
          <div key={nutrient.code}>
            <dt className="text-sm text-gray-600">{nutrient.name}</dt>
            <dd className="tabular-nums text-gray-950">
              {amount}
              {nutrient.goal === null
                ? total.unknown
                  ? ""
                  : ` ${nutrient.unit}`
                : ` / ${f.nutrient(nutrient.goal)} ${nutrient.unit}`}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}

function NutrientTotalsSkeleton() {
  return (
    <div
      role="status"
      aria-label="Loading nutrient totals"
      className="mt-6 grid grid-cols-2 gap-x-4 gap-y-3 motion-safe:animate-pulse sm:grid-cols-4"
    >
      {Array.from({ length: 4 }, (_, index) => (
        <div key={index} aria-hidden="true">
          <div className="flex h-lh items-center text-sm">
            <div className="h-4 w-14 rounded bg-gray-200" />
          </div>
          <div className="flex h-lh items-center">
            <div className="h-4 w-20 rounded bg-gray-250" />
          </div>
        </div>
      ))}
    </div>
  );
}
