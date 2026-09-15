import { AnimatePresence, motion, useIsPresent } from "framer-motion";

import {
  useDeleteWaterMutation,
  type WaterEntryView,
} from "../../api/water.ts";
import { useIsReducedMotion } from "../../lib/use-is-reduced-motion.ts";
import { TrashIcon } from "../../ui/trash-icon.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

export function WaterEntryRow(props: {
  date: string;
  entry: WaterEntryView;
  loading: boolean;
  first: boolean;
  last: boolean;
}) {
  const { f } = useI18n();
  const isReducedMotion = useIsReducedMotion();
  const present = useIsPresent();
  const deletion = useDeleteWaterMutation(props.date);
  const id = props.entry.id;
  const time = f.time(new Date(props.entry.consumed_at));

  return (
    <motion.li
      aria-hidden={props.entry.isDeleting || !present || undefined}
      inert={props.entry.isDeleting || !present}
      layout={isReducedMotion || id === null ? false : "position"}
      initial={{ height: 0, opacity: 0 }}
      animate={{
        height: props.entry.isDeleting ? 0 : "auto",
        opacity: props.entry.isDeleting ? 0 : 1,
      }}
      exit={{ height: 0, opacity: 0 }}
      transition={{
        duration: isReducedMotion ? 0 : 0.25,
        ease: [0.16, 1, 0.3, 1],
      }}
      className="overflow-hidden"
    >
      <div
        className={`relative min-h-11 rounded-lg px-2 text-gray-950 ${props.first ? "" : "mt-1"} ${props.last ? "" : "border-b border-gray-200"}`}
      >
        <motion.div
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 rounded-lg bg-gray-250"
          initial={{ opacity: isReducedMotion ? 0 : 0.55 }}
          animate={{ opacity: 0 }}
          transition={{
            delay: isReducedMotion ? 0 : 0.6,
            duration: isReducedMotion ? 0 : 1.2,
            ease: "easeOut",
          }}
        />
        <div className="relative flex min-h-11 items-center justify-between gap-3">
          <div className="flex items-baseline gap-3">
            <span className="font-medium">
              {f.number(props.entry.amount_ml)} ml
            </span>
            <time
              dateTime={props.entry.consumed_at}
              className="text-sm text-gray-600"
            >
              {time}
            </time>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <AnimatePresence initial={false}>
              {deletion.isError && (
                <motion.p
                  role="alert"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: isReducedMotion ? 0 : 0.16 }}
                  className="rounded-lg bg-danger-surface text-sm text-danger-fg"
                >
                  <span className="block px-2 py-1">Error deleting water</span>
                </motion.p>
              )}
            </AnimatePresence>
            <button
              type="button"
              aria-label={`Delete ${f.number(props.entry.amount_ml)} ml water entry at ${time}`}
              disabled={props.loading || deletion.isPending || id === null}
              className="grid size-9 place-items-center rounded-lg text-danger-fg outline-2 outline-transparent outline-offset-2 hover:not-disabled:bg-danger-surface focus-visible:outline-danger-focus disabled:opacity-50"
              onClick={() => {
                if (id !== null) {
                  deletion.mutate({ id, amount_ml: props.entry.amount_ml });
                }
              }}
            >
              <TrashIcon />
            </button>
          </div>
        </div>
      </div>
    </motion.li>
  );
}
