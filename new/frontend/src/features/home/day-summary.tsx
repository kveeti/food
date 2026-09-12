import type { CSSProperties } from "react";

import {
  type GoalsWithProgress,
  useGoalsProgressQuery,
} from "../../api/goals.ts";
import { Button } from "../../ui/button/button.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { WaterTrigger } from "../water/water-trigger.tsx";

export function DaySummary(props: { date: string }) {
  const goals = useGoalsProgressQuery(props.date);
  const loading = goals.isPending || goals.isPlaceholderData;

  if (goals.isError) {
    return (
      <section aria-label="Day totals" className="mt-3 xl:mt-2">
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
    <section
      aria-label="Day totals"
      aria-busy={loading}
      className="mt-3 xl:mt-2"
    >
      <div
        className={`transition-[filter,opacity] duration-200 ${goals.isPlaceholderData ? "opacity-70 blur-[1px]" : ""}`}
      >
        <div className="@container">
          <div className="grid grid-cols-1 gap-2 @[21rem]:grid-cols-2">
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
              drawerTrigger
              disabled={loading}
            />
          </div>
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
  drawerTrigger?: boolean;
  disabled?: boolean;
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
    unknown || props.goal === null
      ? value
      : `${value} / ${props.tone === "calorie" ? f.calories(props.goal) : f.amount(props.goal)}`;

  const content = (
    <>
      {!props.initialLoading && !unknown && (
        <div aria-hidden="true" className="absolute inset-0">
          <div className="goal-card-fill absolute inset-y-0 left-0 w-full origin-left scale-x-[var(--fill-scale)] bg-[var(--goal-solid)] [transition:transform_360ms_cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none" />
          <div
            className="goal-card-range absolute inset-y-0 w-[var(--range-size)] [border-left:0_solid_var(--goal-soft)] [left:var(--range-start)] [transition:left_360ms_cubic-bezier(0.16,1,0.3,1),width_360ms_cubic-bezier(0.16,1,0.3,1)] after:absolute after:inset-0 after:bg-[var(--goal-solid)] after:opacity-0 after:[background-image:repeating-linear-gradient(135deg,transparent_0_0.35rem,#0000003d_0.35rem_0.7rem)] after:[content:''] after:[transition:opacity_360ms_cubic-bezier(0.16,1,0.3,1)] data-[over-goal=true]:[border-left-width:2px] data-[over-goal=true]:after:opacity-100 motion-reduce:transition-none motion-reduce:after:transition-none"
            data-over-goal={isOverGoal || undefined}
          />
        </div>
      )}
      <div className="relative px-3 py-2">
        <p className="text-xs font-medium text-gray-700">{props.label}</p>
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
            {!unknown && (
              <span className="text-xs font-normal text-gray-700">
                {" "}
                {props.unit}
              </span>
            )}
          </p>
        )}
      </div>
    </>
  );

  const cardProps = {
    className:
      "goal-card relative w-full overflow-hidden rounded-2xl border-0 bg-gray-150 p-0 text-left font-[inherit] data-[has-goal=true]:bg-[var(--goal-soft)] data-[tone=calorie]:[--goal-soft:var(--goal-calorie-soft)] data-[tone=calorie]:[--goal-solid:var(--goal-calorie-solid)] data-[tone=water]:[--goal-soft:var(--goal-water-soft)] data-[tone=water]:[--goal-solid:var(--goal-water-solid)]",
    "data-tone": props.tone,
    "data-has-goal": hasGoal || undefined,
    style,
  };

  if (props.drawerTrigger) {
    return (
      <WaterTrigger
        {...cardProps}
        type="button"
        aria-label="Open water log"
        disabled={props.disabled}
        className={`${cardProps.className} cursor-pointer outline-2 outline-transparent outline-offset-2 focus-visible:outline-gray-500 disabled:cursor-default`}
      >
        {content}
      </WaterTrigger>
    );
  }

  return <div {...cardProps}>{content}</div>;
}

function NutrientTotals(props: { goals: GoalsWithProgress }) {
  const { f } = useI18n();
  const progress = new Map(
    props.goals.progress.nutrients.map((nutrient) => [nutrient.code, nutrient]),
  );

  return (
    <dl className="@container mt-4 flex gap-2">
      {props.goals.nutrients.map((nutrient) => {
        const total = progress.get(nutrient.code);
        if (!total) return null;
        const amount = total.unknown
          ? "Unknown"
          : `${total.incomplete ? "At least " : ""}${f.nutrient(total.eaten)}`;
        return (
          <div
            key={nutrient.code}
            className="nutrient-total min-w-0 flex-1 text-[clamp(0.625rem,3cqw,0.875rem)]"
          >
            <dt className="text-gray-600">{nutrient.name}</dt>
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
      className="@container mt-4 flex gap-2 motion-safe:animate-pulse"
    >
      {Array.from({ length: 4 }, (_, index) => (
        <div
          key={index}
          aria-hidden="true"
          className="nutrient-total min-w-0 flex-1 text-[clamp(0.625rem,3cqw,0.875rem)]"
        >
          <div className="flex h-lh items-center">
            <div className="h-4 w-full max-w-14 rounded bg-gray-200" />
          </div>
          <div className="flex h-lh items-center">
            <div className="h-4 w-full max-w-20 rounded bg-gray-250" />
          </div>
        </div>
      ))}
    </div>
  );
}
