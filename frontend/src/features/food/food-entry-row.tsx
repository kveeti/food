import {
  Field,
  Form,
  type SubmitHandler,
  useField,
  useForm,
} from "@formisch/react";
import { AnimatePresence, motion, useIsPresent } from "framer-motion";
import { useId, useRef, useState } from "react";
import * as v from "valibot";

import {
  type FoodEntry,
  type FoodEntryView,
  useDeleteFoodMutation,
  useUpdateFoodMutation,
} from "../../api/food.ts";
import { useIsReducedMotion } from "../../lib/use-is-reduced-motion.ts";
import { Button } from "../../ui/button/button.tsx";
import { ChevronRightIcon } from "../../ui/chevron-right-icon.tsx";
import { Input } from "../../ui/input/input.tsx";
import { TrashIcon } from "../../ui/trash-icon.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";
import { amountSchema } from "./amount-schema.ts";

const editFoodSchema = v.object({ amount: amountSchema });
export function FoodEntryRow(props: {
  entry: FoodEntryView;
  date: string;
  last: boolean;
}) {
  const [isEditing, setIsEditing] = useState(false);
  const headerButton = useRef<HTMLButtonElement>(null);
  const isReducedMotion = useIsReducedMotion();
  const present = useIsPresent();
  const deletion = useDeleteFoodMutation(props.date);
  const isDeleting = deletion.isPending;
  const savedEntry = props.entry.id === null ? null : props.entry;
  const { f } = useI18n();
  const energy = props.entry.nutrients.find(
    (nutrient) => nutrient.code === "energy",
  );
  const toggle = () => {
    if (isEditing) {
      deletion.reset();
      setIsEditing(false);
      requestAnimationFrame(() => headerButton.current?.focus());
    } else if (savedEntry) {
      setIsEditing(true);
    }
  };

  return (
    <motion.li
      aria-hidden={isDeleting || !present || undefined}
      inert={isDeleting || !present}
      layout={isReducedMotion ? false : "position"}
      initial={{ opacity: 0 }}
      animate={{ height: isDeleting ? 0 : "auto", opacity: isDeleting ? 0 : 1 }}
      exit={{ height: 0, opacity: 0 }}
      transition={{
        duration: isReducedMotion ? 0 : 0.25,
        ease: [0.16, 1, 0.3, 1],
      }}
    >
      <div className="px-[var(--food-row-inset)]">
        <div className="relative py-2">
          <motion.div
            aria-hidden="true"
            className="my-2 pointer-events-none absolute inset-0 rounded-[1.25rem] bg-gray-250"
            initial={{ opacity: isReducedMotion ? 0 : 0.55 }}
            animate={{ opacity: 0 }}
            transition={{
              delay: isReducedMotion ? 0 : 0.6,
              duration: isReducedMotion ? 0 : 1.2,
              ease: "easeOut",
            }}
          />

          <div
            className={`group relative rounded-[1.25rem] transition-colors duration-200 motion-reduce:transition-none ${isEditing ? "bg-gray-100" : "hover:bg-gray-100"}`}
          >
            <div className="grid grid-cols-[minmax(0,1fr)_7rem_auto] grid-rows-[minmax(4.5rem,auto)_auto] items-start gap-x-1.5">
              <button
                ref={headerButton}
                type="button"
                aria-label={`${isEditing ? "Close" : "Edit"} ${props.entry.food_name} entry`}
                onClick={toggle}
                disabled={!savedEntry}
                className="col-span-3 col-start-1 row-start-1 grid self-stretch grid-cols-subgrid rounded-[1.25rem] text-left font-[inherit] outline-2 outline-transparent outline-offset-2 focus-visible:outline-gray-500"
              >
                <span className="col-start-1 row-start-1 flex min-w-0 items-start justify-start pt-3 pb-2 pl-[calc(var(--page-padding)-var(--food-row-inset))]">
                  <FoodIdentity entry={props.entry} />
                </span>

                <AnimatePresence initial={false} mode="sync">
                  {!isEditing && (
                    <motion.span
                      key="amount-summary"
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: isReducedMotion ? 0 : 0.16 }}
                      className="col-start-2 row-start-1 py-3 text-right tabular-nums"
                    >
                      <span className="block font-medium text-gray-700">
                        {f.amount(props.entry.amount)} {props.entry.unit}
                      </span>
                      <span className="block text-sm text-gray-600">
                        {energy
                          ? `${f.calories(energy.value)} kcal`
                          : "-- kcal"}
                      </span>
                    </motion.span>
                  )}
                </AnimatePresence>

                <span className="col-start-3 row-start-1 grid self-stretch place-items-center pr-1.5 text-gray-600">
                  <span className="grid size-6 place-items-center">
                    <ChevronRightIcon
                      className={`transition-transform duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none ${isEditing ? "rotate-90" : ""}`}
                    />
                  </span>
                </span>
              </button>

              <AnimatePresence initial={false} mode="sync">
                {isEditing && savedEntry && (
                  <FoodEntryEditor
                    key="editor"
                    entry={savedEntry}
                    date={props.date}
                    reducedMotion={isReducedMotion}
                    deletion={deletion}
                    onClose={toggle}
                  />
                )}
              </AnimatePresence>
            </div>
          </div>
          {!props.last && (
            <div
              aria-hidden="true"
              className="pointer-events-none absolute inset-x-5 bottom-0 border-b border-gray-200"
            />
          )}
        </div>
      </div>
    </motion.li>
  );
}

function FoodIdentity(props: { entry: FoodEntryView }) {
  return (
    <span className="min-w-0">
      <span className="line-clamp-2 font-medium text-gray-950">
        {props.entry.food_name}
      </span>
      {props.entry.food_brand && (
        <span className="mt-0.5 block text-sm text-gray-600">
          {props.entry.food_brand}
        </span>
      )}
    </span>
  );
}

function FoodEntryEditor(props: {
  entry: FoodEntry;
  date: string;
  reducedMotion: boolean;
  deletion: ReturnType<typeof useDeleteFoodMutation>;
  onClose: () => void;
}) {
  const formId = useId();
  const form = useForm({
    schema: editFoodSchema,
    initialInput: { amount: String(props.entry.amount) },
  });
  const amount = useField(form, { path: ["amount"] });
  const update = useUpdateFoodMutation(props.date);
  const deletion = props.deletion;

  const onSubmit: SubmitHandler<typeof editFoodSchema> = async (values) => {
    if (update.isPending || deletion.isPending) return;
    try {
      await update.mutateAsync({ entry: props.entry, amount: values.amount });
      props.onClose();
    } catch {
      /* The mounted editor keeps the mutation error visible. */
    }
  };

  return (
    <>
      <motion.div
        key="amount-input"
        initial={{ height: 0, opacity: 0 }}
        animate={{ height: "auto", opacity: 1 }}
        exit={{ height: 0, opacity: 0 }}
        transition={{
          height: {
            duration: props.reducedMotion ? 0 : 0.22,
            ease: [0.16, 1, 0.3, 1],
          },
          opacity: {
            duration: props.reducedMotion ? 0 : 0.14,
            ease: "easeOut",
          },
        }}
        className="col-start-2 row-start-1 w-28"
      >
        <div className="pt-3">
          <Field of={form} path={["amount"]}>
            {(field) => (
              <div>
                <div className="relative">
                  <Input
                    {...field.props}
                    form={formId}
                    value={field.input ?? ""}
                    aria-label={`Amount (${props.entry.unit})`}
                    error={!!field.errors}
                    inputMode="decimal"
                    autoComplete="off"
                    autoFocus
                    className="pr-7"
                    size="small"
                  />
                  <span
                    aria-hidden="true"
                    className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-sm text-gray-600"
                  >
                    {props.entry.unit}
                  </span>
                </div>
                {field.errors && (
                  <span className="mt-1.5 block text-sm text-[var(--input-invalid-text)]">
                    {field.errors[0]}
                  </span>
                )}
              </div>
            )}
          </Field>
          <LiveKcal amount={amount.input ?? ""} entry={props.entry} />
        </div>
      </motion.div>

      <motion.div
        key="editor-actions"
        initial={{ height: 0, opacity: 0 }}
        animate={{ height: "auto", opacity: 1 }}
        exit={{ height: 0, opacity: 0 }}
        transition={{
          height: {
            duration: props.reducedMotion ? 0 : 0.22,
            ease: [0.16, 1, 0.3, 1],
          },
          opacity: {
            duration: props.reducedMotion ? 0 : 0.14,
            ease: "easeOut",
          },
        }}
        className="col-span-3 col-start-1 row-start-2 overflow-hidden"
      >
        <Form
          id={formId}
          of={form}
          onSubmit={onSubmit}
          className="space-y-3 p-2"
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <Button
                type="button"
                variant="destructive"
                aria-label={`Delete ${props.entry.food_name} entry`}
                className="px-3!"
                onClick={() => {
                  if (update.isPending || deletion.isPending) return;
                  deletion.mutate(props.entry.id);
                }}
              >
                <TrashIcon />
                Delete
              </Button>
              <AnimatePresence initial={false}>
                {deletion.isError && (
                  <motion.p
                    role="alert"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: props.reducedMotion ? 0 : 0.16 }}
                    className="rounded-lg bg-danger-surface text-sm text-danger-fg"
                  >
                    <span className="block px-2 py-1">Error deleting food</span>
                  </motion.p>
                )}
              </AnimatePresence>
            </div>
            <div className="ml-auto flex flex-wrap justify-end gap-2">
              <AnimatePresence initial={false}>
                {update.isError && (
                  <motion.p
                    role="alert"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: props.reducedMotion ? 0 : 0.16 }}
                    className="rounded-lg bg-danger-surface text-sm text-danger-fg"
                  >
                    <span className="block px-2 py-1">Error updating food</span>
                  </motion.p>
                )}
              </AnimatePresence>
              <Button type="button" variant="ghost" onClick={props.onClose}>
                Cancel
              </Button>
              <Button type="submit">Save</Button>
            </div>
          </div>
        </Form>
      </motion.div>
    </>
  );
}

function LiveKcal(props: { amount: string; entry: FoodEntry }) {
  const { f } = useI18n();
  const number = Number(props.amount.replace(",", "."));
  const energy = props.entry.nutrients.find(
    (nutrient) => nutrient.code === "energy",
  );

  let value: string | number | null = null;
  if (!energy || !Number.isFinite(number) || number <= 0) {
    value = "--";
  } else {
    value = f.calories((energy.value * number) / props.entry.amount);
  }

  return (
    <output className="mt-1 block max-w-full text-right text-sm text-gray-600">
      <span className="truncate tabular-nums">{value}</span> kcal
    </output>
  );
}
