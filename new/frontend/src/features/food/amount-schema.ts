import * as v from "valibot";

export const amountSchema = v.pipe(
  v.string(),
  v.trim(),
  v.nonEmpty("Enter an amount"),
  v.transform((value) => Number(value.replace(",", "."))),
  v.number("Enter a number"),
  v.finite("Enter a finite amount"),
  v.minValue(Number.MIN_VALUE, "Amount must be greater than 0"),
  v.maxValue(100_000, "Amount must be at most 100000"),
);
