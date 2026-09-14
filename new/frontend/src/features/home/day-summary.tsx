import {
  type GoalsWithProgress,
  useGoalsProgressQuery,
} from "../../api/goals.ts";
import { Button } from "../../ui/button/button.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { GoalCard } from "./goal-card.tsx";

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
