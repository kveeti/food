import { useGoalsProgressQuery } from "../../api/goals.ts";
import { useI18n } from "../i18n/use-i18n.tsx";

export function NutrientTotals(props: { date: string }) {
  const goals = useGoalsProgressQuery(props.date);
  const { f } = useI18n();

  if (goals.isPending) return null;
  if (goals.isError) {
    return (
      <p role="alert" className="mb-5 text-danger-fg">
        Could not load nutrient goals.
      </p>
    );
  }
  const progress = new Map(
    goals.data.progress.nutrients.map((nutrient) => [nutrient.code, nutrient]),
  );

  return (
    <dl className="mt-6 grid grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-4">
      {goals.data.nutrients.map((nutrient) => {
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
