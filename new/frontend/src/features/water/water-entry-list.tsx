import { useWaterEntriesQuery } from "../../api/water.ts";
import { WaterEntryRow } from "./water-entry-row.tsx";

export function WaterEntryList(props: { date: string }) {
  const waterEntries = useWaterEntriesQuery(props.date);
  const loading = waterEntries.isPending || waterEntries.isPlaceholderData;

  return (
    <>
      {waterEntries.isError && (
        <p role="alert" className="mt-3 text-base text-danger-fg">
          Could not load the water entries.
        </p>
      )}
      <ul
        aria-label="Water entries"
        aria-busy={loading}
        className={`mt-4 space-y-1 transition-[filter,opacity] duration-200 ${loading ? "pointer-events-none opacity-60 blur-[2px]" : ""}`}
      >
        {waterEntries.entries.map((entry) => (
          <WaterEntryRow
            key={entry.id === null ? entry.submittedAt : entry.id}
            date={props.date}
            entry={entry}
            loading={loading}
          />
        ))}
      </ul>
    </>
  );
}
