import { useWaterEntriesQuery } from "../../api/water.ts";
import { useI18n } from "../i18n/use-i18n.tsx";

export function WaterTotal(props: { date: string }) {
  const waterEntries = useWaterEntriesQuery(props.date);
  const { f } = useI18n();
  const loading = waterEntries.isPending || waterEntries.isPlaceholderData;
  const total = waterEntries.entries
    .filter((entry) => !entry.isDeleting)
    .reduce((sum, entry) => sum + entry.amount_ml, 0);
  const unknown =
    waterEntries.isError ||
    (waterEntries.isPending && waterEntries.entries.length === 0);

  return (
    <p
      aria-label="Water total"
      aria-busy={loading}
      className={`text-xl font-semibold tabular-nums transition-[filter] duration-200 ${loading ? "blur-[2px]" : ""}`}
    >
      {unknown ? "--" : `${f.number(total)} ml`}
    </p>
  );
}
