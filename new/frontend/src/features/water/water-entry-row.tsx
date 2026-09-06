import { useEffect } from "react";

import { useDeleteWaterMutation, type WaterEntry } from "../../api/water.ts";
import { TrashIcon } from "../../ui/trash-icon.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

export function WaterEntryRow(props: {
  date: string;
  entry: Omit<WaterEntry, "id"> & { id: string | null; isDeleting: boolean };
  loading: boolean;
}) {
  const { f } = useI18n();
  const deletion = useDeleteWaterMutation(props.date);
  const { isError, reset } = deletion;
  const id = props.entry.id;
  const time = f.time(new Date(props.entry.consumed_at));

  useEffect(() => {
    if (!isError) return;
    const timeout = window.setTimeout(reset, 1_500);
    return () => window.clearTimeout(timeout);
  }, [isError, reset]);

  return (
    <li
      hidden={props.entry.isDeleting}
      className={`min-h-11 rounded-lg border-b border-gray-200 px-2 transition-colors duration-200 last:border-b-0 starting:bg-transparent ${isError ? "bg-danger-surface text-danger-fg" : "text-gray-950"}`}
    >
      <div className="flex min-h-11 items-center justify-between gap-3">
        <div className="flex items-baseline gap-3">
          <span className="font-medium">
            {f.number(props.entry.amount_ml)} ml
          </span>
          <time
            dateTime={props.entry.consumed_at}
            className={`text-sm ${isError ? "text-danger-fg" : "text-gray-600"}`}
          >
            {time}
          </time>
        </div>
        <div className="relative shrink-0">
          <button
            type="button"
            aria-label={`Delete ${f.number(props.entry.amount_ml)} ml water entry at ${time}`}
            disabled={props.loading || deletion.isPending || id === null}
            className="grid size-9 place-items-center rounded-lg text-danger-fg outline-2 outline-transparent outline-offset-2 hover:not-disabled:bg-danger-surface focus-visible:outline-danger-focus disabled:opacity-50"
            onClick={() => {
              if (id !== null) deletion.mutate(id);
            }}
          >
            <TrashIcon />
          </button>
          <p
            role="alert"
            aria-hidden={!isError}
            className={`pointer-events-none absolute top-1/2 right-full z-10 mr-2 -translate-y-1/2 rounded-md px-2 py-1 text-sm whitespace-nowrap text-danger-fg transition-opacity duration-200 starting:opacity-0 ${isError ? "opacity-100" : "opacity-0"}`}
          >
            error deleting entry
          </p>
        </div>
      </div>
    </li>
  );
}
