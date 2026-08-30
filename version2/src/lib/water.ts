import { action, query } from "@solidjs/router";
import { db } from "../lib/data/db";
import { waterLog } from "../lib/data/schema";
import { getCurrentUser } from "../lib/user";
import { isoDateTime, minValue, number, object, parseAsync, pipe, string, transform, trim } from "valibot";
import { and, eq, sql } from "drizzle-orm";

export const logWater = action(async (formData: FormData) => {
  "use server";
  const user = await getCurrentUser();

  const data = await parseAsync(waterForm, {
    amountMl: formData.get("amountMl"),
    consumedAt: formData.get("consumedAt"),
  });

  console.log("logWater", data)

  await db.insert(waterLog).values({
    amountMl: data.amountMl,
    consumedAt: data.consumedAt,
    userId: user.id,
  });

  console.log("logWater ok", data)

  throw new Error("oh no")

  return "logged"
}, "logWater");

const waterForm = object({
  amountMl: pipe(string(), trim(), transform(v => Number(v)), number(), minValue(1)),
  consumedAt : pipe(string(), trim(), isoDateTime(), transform(v => new Date(v)))
});

export const getWaterMlToday = query(async () => {
  "use server";
  const user = await getCurrentUser();

  const [waterToday] = await db
  .select({
    totalMl: sql<number>`sum(${waterLog.amountMl})`,
  })
  .from(waterLog)
  .where(
    and(
      eq(waterLog.userId, user.id),
      sql`(${waterLog.consumedAt} AT TIME ZONE ${user.timezone})::date = (now() AT TIME ZONE ${user.timezone})::date`
    )
  );

  const totalMl = waterToday?.totalMl ?? 0;
  return new Intl.NumberFormat("en-US", {notation: "compact"}).format(totalMl)
}, "getWater");

