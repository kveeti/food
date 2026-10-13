import { AnimatePresence } from "framer-motion";

import { useWaterEntriesQuery } from "../../api/water.ts";
import { Button } from "../../ui/button/button.tsx";
import { WaterEntryRow } from "./water-entry-row.tsx";

export function WaterEntryList(props: { date: string }) {
  const waterEntries = useWaterEntriesQuery(props.date);

  return (
    <div
      role="region"
      aria-label="Water history"
      className="mt-4 min-h-[calc(12rem+max(1rem,env(safe-area-inset-bottom)))] flex-1 overflow-y-auto overscroll-contain"
    >
      <div className="pb-[max(1rem,env(safe-area-inset-bottom))]">
        {waterEntries.isPending ? (
          <WaterEntriesSkeleton />
        ) : waterEntries.isError ? (
          <WaterEntriesError
            retrying={waterEntries.isFetching}
            onRetry={() => void waterEntries.refetch()}
          />
        ) : (
          <div
            aria-busy={waterEntries.isPlaceholderData}
            inert={waterEntries.isPlaceholderData}
            className={`transition-[filter,opacity] duration-200 ${waterEntries.isPlaceholderData ? "pointer-events-none opacity-70 blur-[1px]" : ""}`}
          >
            <WaterEntries
              key={`${props.date}:${waterEntries.isPlaceholderData ? "stale" : "current"}`}
              date={props.date}
              entries={waterEntries.entries}
              loading={waterEntries.isPlaceholderData}
            />
            {waterEntries.entries.every((entry) => entry.isDeleting) && (
              <WaterEmptyState />
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function WaterEntries(props: {
  date: string;
  entries: ReturnType<typeof useWaterEntriesQuery>["entries"];
  loading: boolean;
}) {
  const visibleEntries = props.entries.filter((entry) => !entry.isDeleting);
  const firstVisibleEntry = visibleEntries[0];
  const lastVisibleEntry = visibleEntries.at(-1);

  return (
    <ul aria-label="Water entries">
      <AnimatePresence initial={false}>
        {props.entries.map((entry, index) => (
          <WaterEntryRow
            key={entry.renderKey}
            date={props.date}
            entry={entry}
            loading={props.loading}
            first={index === 0 || entry === firstVisibleEntry}
            last={
              index === props.entries.length - 1 || entry === lastVisibleEntry
            }
          />
        ))}
      </AnimatePresence>
    </ul>
  );
}

function WaterEmptyState() {
  return (
    <div className="rounded-2xl bg-gray-100">
      <div className="flex items-center gap-3 p-4">
        <div className="grid size-10 shrink-0 place-items-center rounded-full bg-gray-200 text-gray-600">
          <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="size-5"
          >
            <path d="M12 3.5S6.5 9.5 6.5 14a5.5 5.5 0 0 0 11 0C17.5 9.5 12 3.5 12 3.5Z" />
          </svg>
        </div>
        <div>
          <p className="font-medium text-gray-950">
            No water logged for this day
          </p>
        </div>
      </div>
    </div>
  );
}

function WaterEntriesError(props: { retrying: boolean; onRetry: () => void }) {
  return (
    <div className="rounded-2xl bg-danger-surface">
      <div className="flex items-center justify-between gap-4 p-4">
        <p role="alert" className="font-medium text-danger-fg">
          Error loading water entries
        </p>
        <Button
          type="button"
          variant="outline"
          disabled={props.retrying}
          onClick={props.onRetry}
        >
          {props.retrying ? "Retrying…" : "Try again"}
        </Button>
      </div>
    </div>
  );
}

function WaterEntriesSkeleton() {
  return (
    <div
      role="status"
      aria-label="Loading water entries"
      className="space-y-1 motion-safe:animate-pulse"
    >
      {Array.from({ length: 2 }, (_, index) => (
        <div
          key={index}
          aria-hidden="true"
          className="flex min-h-11 items-center justify-between gap-3 border-b border-gray-200 px-2"
        >
          <div className="h-5 w-24 rounded bg-gray-200" />
          <div className="size-9 rounded-lg bg-gray-200" />
        </div>
      ))}
    </div>
  );
}
