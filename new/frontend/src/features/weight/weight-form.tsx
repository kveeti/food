import { Field as FormField, Form, setInput, useForm } from "@formisch/react";
import { Temporal } from "@js-temporal/polyfill";
import { useState, type RefObject } from "react";
import * as v from "valibot";

import { type WeightEntry, type useWeightMutation } from "../../api/weight.ts";
import { createId } from "../../lib/id.ts";
import { Button } from "../../ui/button/button.tsx";
import { Field } from "../../ui/input/field.tsx";
import { Input } from "../../ui/input/input.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

const weightSchema = v.pipe(
  v.string(),
  v.trim(),
  v.nonEmpty("Enter a weight"),
  v.regex(/^\d+(?:[.,]\d+)?$/, "Enter a weight in kg"),
  v.transform((value) => Number(value.replace(",", "."))),
  v.number("Enter a number"),
  v.finite("Enter a finite weight"),
  v.minValue(Number.MIN_VALUE, "Weight must be greater than 0"),
);

export function WeightForm(props: {
  entry: WeightEntry | null;
  inputRef: RefObject<HTMLInputElement | null>;
  mutation: ReturnType<typeof useWeightMutation>;
  onDone: () => void;
}) {
  const { timeZone } = useI18n();
  const [initialInstant, setInitialInstant] = useState(
    () => props.entry?.measured_at ?? new Date().toISOString(),
  );
  const initialTime = Temporal.Instant.from(initialInstant)
    .toZonedDateTimeISO(timeZone)
    .toPlainDateTime()
    .toString({ smallestUnit: "minute" });
  const mutation = props.mutation;

  function measurementTime(value: string) {
    if (value === initialTime) return initialInstant;
    try {
      const local = Temporal.PlainDateTime.from(value);
      if (local.year < 1 || local.year > 9999) return undefined;
      return local
        .toZonedDateTime(timeZone, { disambiguation: "reject" })
        .toInstant()
        .toString();
    } catch {
      return undefined;
    }
  }

  const form = useForm({
    schema: v.object({
      weight: weightSchema,
      measuredAt: v.pipe(
        v.string(),
        v.nonEmpty("Enter a date and time"),
        v.check(
          (value) => measurementTime(value) !== undefined,
          "Choose a valid date and time. Clock changes can skip or repeat times.",
        ),
      ),
    }),
    initialInput: {
      weight: props.entry ? String(props.entry.weight_kg) : "",
      measuredAt: initialTime,
    },
  });

  return (
    <Form
      of={form}
      className="space-y-5"
      onSubmit={({ weight, measuredAt }) => {
        if (mutation.isPending) return;
        const measured_at = measurementTime(measuredAt);
        if (!measured_at) return;
        mutation.mutate(
          {
            kind: props.entry ? "update" : "add",
            entry: {
              id: props.entry?.id ?? createId(),
              weight_kg: weight,
              measured_at,
            },
          },
          { onSuccess: props.onDone },
        );
      }}
    >
      <FormField of={form} path={["weight"]}>
        {(field) => (
          <Field label="Weight (kg)" error={field.errors?.[0]}>
            <Input
              {...field.props}
              ref={props.inputRef}
              value={field.input ?? ""}
              error={!!field.errors}
              inputMode="decimal"
              autoComplete="off"
              readOnly={mutation.isPending}
            />
          </Field>
        )}
      </FormField>
      <FormField of={form} path={["measuredAt"]}>
        {(field) => (
          <div className="flex flex-col gap-1.5">
            <span className="text-sm text-gray-700">Date and time</span>
            <div className="flex items-center gap-2">
              <Input
                {...field.props}
                aria-label="Date and time"
                type="datetime-local"
                min="0001-01-01T00:00"
                max="9999-12-31T23:59"
                value={field.input ?? ""}
                error={!!field.errors}
                readOnly={mutation.isPending}
              />
              <Button
                type="button"
                variant="outline"
                className="h-10 shrink-0"
                disabled={mutation.isPending}
                onClick={() => {
                  const now = Temporal.Now.instant();
                  setInitialInstant(now.toString());
                  setInput(form, {
                    path: ["measuredAt"],
                    input: now
                      .toZonedDateTimeISO(timeZone)
                      .toPlainDateTime()
                      .toString({ smallestUnit: "minute" }),
                  });
                }}
              >
                Now
              </Button>
            </div>
            {field.errors?.[0] && (
              <span className="text-sm text-[var(--input-invalid-text)]">
                {field.errors[0]}
              </span>
            )}
          </div>
        )}
      </FormField>
      {mutation.isError && (
        <p role="alert" className="text-sm text-danger-fg">
          {mutation.variables.kind === "delete"
            ? "Could not delete weight. Try again."
            : "Could not save weight. Try again."}
        </p>
      )}
      <div className="flex flex-wrap justify-end gap-2">
        {props.entry && (
          <Button
            type="button"
            variant="destructive"
            className="mr-auto"
            disabled={mutation.isPending}
            onClick={() => {
              mutation.mutate(
                { kind: "delete", entry: props.entry! },
                { onSuccess: props.onDone },
              );
            }}
          >
            Delete
          </Button>
        )}
        {props.entry && (
          <Button
            type="button"
            variant="ghost"
            disabled={mutation.isPending}
            onClick={props.onDone}
          >
            Cancel
          </Button>
        )}
        <Button type="submit" disabled={mutation.isPending}>
          {mutation.isPending && mutation.variables.kind !== "delete"
            ? "Saving…"
            : "Save"}
        </Button>
      </div>
    </Form>
  );
}
