import type { CSSProperties } from "react";

import { useGoalsProgressQuery } from "../../api/goals.ts";
import { useI18n } from "../i18n/use-i18n.tsx";

export function DaySummary(props: { date: string }) {
  const goals = useGoalsProgressQuery(props.date);

  return (
    <section aria-label="Day totals" className="mt-6 grid grid-cols-2 gap-3">
      <GoalCard
        label="Calories"
        ariaLabel="Calorie total"
        value={goals.data?.progress.calories.eaten ?? 0}
        goal={goals.data?.calorie_goal_kcal ?? null}
        unit="kcal"
        tone="calorie"
        incomplete={goals.data?.progress.calories.incomplete}
        valueUnknown={goals.data?.progress.calories.unknown}
        valueLoading={goals.isPending}
        loading={goals.isPending}
        error={goals.isError}
      />
      <GoalCard
        label="Water"
        ariaLabel="Water total"
        value={goals.data?.progress.water_ml ?? 0}
        goal={goals.data?.water_ml ?? null}
        unit="ml"
        tone="water"
        valueLoading={goals.isPending}
        loading={goals.isPending || goals.isPlaceholderData}
        error={goals.isError}
      />
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
  valueLoading: boolean;
  loading: boolean;
  error: boolean;
}) {
  const { f } = useI18n();
  const chartMax = Math.max(props.value, props.goal ?? 0, 1);
  const goalPercent = props.goal === null ? 0 : (props.goal / chartMax) * 100;
  const fillPercent =
    (Math.min(props.value, props.goal ?? props.value) / chartMax) * 100;
  const overflowPercent =
    props.goal === null
      ? 0
      : (Math.max(0, props.value - props.goal) / chartMax) * 100;
  const isOverGoal = overflowPercent > 0;
  const style = {
    "--fill-scale": fillPercent / 100,
    "--range-start": `${isOverGoal ? goalPercent : fillPercent}%`,
    "--range-size": `${isOverGoal ? overflowPercent : 100 - fillPercent}%`,
  } as CSSProperties;
  const unavailable = props.error || (props.valueLoading && props.value === 0);
  const unknown = props.valueUnknown && !props.valueLoading;
  const value = unavailable
    ? "--"
    : unknown
      ? "Unknown"
      : `${props.incomplete ? "At least " : ""}${props.tone === "calorie" ? f.calories(props.value) : f.amount(props.value)}`;
  const text =
    props.goal === null || unavailable
      ? `${value}${unavailable || unknown ? "" : ` ${props.unit}`}`
      : `${value}${unknown ? "" : ` / ${props.tone === "calorie" ? f.calories(props.goal) : f.amount(props.goal)} ${props.unit}`}`;

  return (
    <div
      className="goal-card relative overflow-hidden rounded-2xl bg-gray-150"
      data-tone={props.tone}
      data-has-goal={props.goal !== null || undefined}
      style={style}
    >
      {props.goal !== null && !unavailable && !unknown && (
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
        <p
          aria-label={props.ariaLabel}
          aria-busy={props.loading}
          className="mt-1 font-medium tabular-nums text-gray-950"
        >
          {text}
        </p>
      </div>
    </div>
  );
}
