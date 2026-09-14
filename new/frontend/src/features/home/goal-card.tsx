import type { CSSProperties } from "react";

import { useI18n } from "../i18n/use-i18n.tsx";
import { WaterTrigger } from "../water/water-trigger.tsx";

export function GoalCard(props: {
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
          <div className="goal-card-fill absolute inset-y-0 left-0 w-full origin-left scale-x-[var(--fill-scale)] bg-[var(--goal-solid)] [transition:scale_360ms_cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none" />
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
