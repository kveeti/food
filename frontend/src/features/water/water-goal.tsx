import { useGoalsProgressQuery } from "../../api/goals.ts";
import { Button } from "../../ui/button/button.tsx";
import { GoalCard } from "../home/goal-card.tsx";

export function WaterGoal(props: { date: string }) {
  const goals = useGoalsProgressQuery(props.date);

  if (goals.isError) {
    return (
      <div className="shrink-0 rounded-2xl bg-danger-surface">
        <div className="flex items-center justify-between gap-3 p-3">
          <p role="alert" className="text-sm text-danger-fg">
            Error loading water goal
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
    );
  }

  return (
    <section
      aria-label="Water goal"
      aria-busy={goals.isPending || goals.isPlaceholderData}
      className={`shrink-0 transition-[filter,opacity] duration-200 ${goals.isPlaceholderData ? "opacity-70 blur-[1px]" : ""}`}
    >
      <GoalCard
        label="Water"
        ariaLabel="Water goal progress"
        value={goals.data?.progress.water_ml ?? 0}
        goal={goals.data?.water_ml ?? null}
        unit="ml"
        tone="water"
        initialLoading={goals.isPending}
      />
    </section>
  );
}
