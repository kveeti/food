import {
  AnimatePresence,
  motion,
  useIsPresent,
  useReducedMotion,
} from "framer-motion";
import { useRef, useState } from "react";

import {
  type WeightEntry,
  useWeightEntriesQuery,
  useWeightMutation,
} from "../../api/weight.ts";
import { Button } from "../../ui/button/button.tsx";
import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { WeightForm } from "./weight-form.tsx";

export function WeightSection() {
  const { f, timeZone } = useI18n();
  const query = useWeightEntriesQuery();
  const mutation = useWeightMutation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [entry, setEntry] = useState<WeightEntry | null>(null);
  const [formKey, setFormKey] = useState(0);
  const [isHistoryRequestedOpen, setIsHistoryRequestedOpen] = useState(false);
  const latest = query.entries[0];
  const isHistoryOpen = isHistoryRequestedOpen && query.entries.length > 0;

  function resetForm() {
    setEntry(null);
    setFormKey((key) => key + 1);
  }

  return (
    <section aria-label="Weight" className="mb-10">
      <h2 className="text-lg font-semibold text-gray-950">Weight</h2>
      <div className="mt-3">
        <p aria-label="Latest weight" className="text-sm text-gray-600">
          <span className="font-medium text-gray-950">Latest weight</span>
          {latest ? (
            <>
              {" · "}
              <span className="font-medium tabular-nums text-gray-950">
                {f.amount(latest.weight_kg)} kg
              </span>
              {" · "}
              <MeasurementTime entry={latest} />
            </>
          ) : (
            " · No weight yet"
          )}
        </p>

        {query.isError && (
          <div className="mt-3 flex items-center justify-between gap-3">
            <p role="alert" className="text-sm text-danger-fg">
              Could not load weight history.
            </p>
            <Button
              type="button"
              variant="ghost"
              onClick={() => void query.refetch()}
            >
              Try again
            </Button>
          </div>
        )}

        <div className="mt-6">
          <WeightForm
            key={`${entry?.id ?? "new"}-${formKey}-${timeZone}`}
            entry={entry}
            inputRef={inputRef}
            mutation={mutation}
            onDone={resetForm}
          />
        </div>

        <div className="-mx-[var(--page-padding)] mt-6 sm:rounded-[calc(var(--radius-xl)+0.5rem)]">
          <h3 className="rounded-[inherit]">
            <button
              type="button"
              aria-label={`${isHistoryOpen ? "Close" : "Open"} weight history`}
              aria-expanded={isHistoryOpen}
              aria-controls="weight-history"
              disabled={query.entries.length === 0}
              onClick={() => setIsHistoryRequestedOpen(!isHistoryOpen)}
              className="block w-full rounded-[inherit] text-left font-[inherit] outline-2 outline-transparent outline-offset-[-2px] hover:not-disabled:bg-gray-150 focus-visible:outline-gray-500 disabled:cursor-default disabled:opacity-60"
            >
              <span className="flex min-h-12 items-center gap-3 px-[var(--page-padding)] py-2">
                <span className="min-w-0 flex-1 font-medium text-gray-950">
                  Weight history
                </span>
                <span className="grid size-6 shrink-0 place-items-center text-gray-600">
                  <ChevronRightIcon
                    className={`transition-transform duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none ${isHistoryOpen ? "rotate-90" : ""}`}
                  />
                </span>
              </span>
            </button>
          </h3>
          <AnimatePresence initial={false}>
            {isHistoryOpen && query.entries.length > 0 && (
              <WeightHistory
                entries={query.entries.slice(0, 10)}
                isPending={mutation.isPending}
                onSelect={(historyEntry) => {
                  mutation.reset();
                  setEntry(historyEntry);
                  requestAnimationFrame(() => inputRef.current?.focus());
                }}
              />
            )}
          </AnimatePresence>
        </div>
      </div>
    </section>
  );
}

function WeightHistory(props: {
  entries: WeightEntry[];
  isPending: boolean;
  onSelect: (entry: WeightEntry) => void;
}) {
  const { f } = useI18n();
  const isPresent = useIsPresent();
  const isReducedMotion = useReducedMotion();

  return (
    <motion.div
      id="weight-history"
      inert={!isPresent}
      aria-hidden={!isPresent || undefined}
      initial={{ height: 0, opacity: 0 }}
      animate={{ height: "auto", opacity: 1 }}
      exit={{ height: 0, opacity: 0 }}
      transition={{
        height: {
          duration: isReducedMotion ? 0 : 0.22,
          ease: [0.16, 1, 0.3, 1],
        },
        opacity: { duration: isReducedMotion ? 0 : 0.14, ease: "easeOut" },
      }}
      className="overflow-hidden"
    >
      <div className="px-[var(--page-padding)] pt-2">
        <ul aria-label="Weight history" className="divide-y divide-gray-200">
          {props.entries.map((entry) => (
            <li key={entry.id}>
              <div className="py-1">
                <button
                  type="button"
                  disabled={props.isPending}
                  className="block w-full cursor-pointer rounded-xl text-left outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-100 focus-visible:outline-gray-500 disabled:cursor-default disabled:opacity-60"
                  onClick={() => props.onSelect(entry)}
                >
                  <span className="flex items-center justify-between gap-3 py-3">
                    <span className="min-w-0">
                      <span className="block font-medium tabular-nums text-gray-950">
                        {f.amount(entry.weight_kg)} kg
                      </span>
                      <span className="mt-0.5 block text-sm text-gray-600">
                        <MeasurementTime entry={entry} />
                      </span>
                    </span>
                    <span className="sr-only">Edit</span>
                    <ChevronRightIcon className="shrink-0 text-gray-500" />
                  </span>
                </button>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </motion.div>
  );
}

function MeasurementTime(props: { entry: WeightEntry }) {
  const { f } = useI18n();
  const measuredAt = new Date(props.entry.measured_at);

  return (
    <time dateTime={props.entry.measured_at}>
      {f.dateOnly(f.dateKey(measuredAt))} · {f.time(measuredAt)}
    </time>
  );
}
