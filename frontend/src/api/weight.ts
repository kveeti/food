import { Temporal } from "@js-temporal/polyfill";
import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { api } from "./api.ts";

export type WeightPoint = {
  weight_kg: number;
  measured_at: string;
  trend_weight_kg: number;
};

export type WeightChartResponse = {
  points: WeightPoint[];
  first: WeightEntry | null;
  last: WeightEntry | null;
  latest: WeightEntry | null;
};

export type WeightEntry = {
  id: string;
  weight_kg: number;
  measured_at: string;
};

type WeightChange = {
  kind: "add" | "update" | "delete";
  entry: WeightEntry;
};

const historyLimit = 10;
const weightEntriesQueryKey = ["weight-entries", historyLimit] as const;
const weightChartQueryKey = ["weight-chart"] as const;
const mutationKey = ["weight-entry"] as const;

function recentEntries(entries: WeightEntry[]) {
  return entries
    .toSorted(
      (a, b) =>
        Date.parse(b.measured_at) - Date.parse(a.measured_at) ||
        b.id.localeCompare(a.id),
    )
    .slice(0, historyLimit);
}

export function useWeightEntriesQuery() {
  return useQuery({
    queryKey: weightEntriesQueryKey,
    queryFn: ({ signal }) =>
      api<WeightEntry[]>(`/api/weight-entries?limit=${historyLimit}`, {
        signal,
      }),
  });
}

export function useWeightChartQuery(
  range: string,
  timeZone: string,
  maxPoints: number,
) {
  const localDay = useLocalDay(timeZone);
  const { from, to } = chartRangeBounds(range, timeZone, localDay);
  const query = new URLSearchParams({
    to,
    timezone: timeZone,
    max_points: String(maxPoints),
  });
  if (from) query.set("from", from);
  return useQuery({
    queryKey: [...weightChartQueryKey, range, from, to, timeZone, maxPoints],
    queryFn: ({ signal }) =>
      api<WeightChartResponse>(`/api/weight-chart?${query}`, { signal }),
    placeholderData: keepPreviousData,
  });
}

function useLocalDay(timeZone: string) {
  const [savedDay, setSavedDay] = useState(() => ({
    timeZone,
    day: currentLocalDay(timeZone),
  }));
  const day =
    savedDay.timeZone === timeZone ? savedDay.day : currentLocalDay(timeZone);

  useEffect(() => {
    let timeout: number;
    const refreshDay = () => {
      const currentDay = currentLocalDay(timeZone);
      setSavedDay((saved) =>
        saved.timeZone === timeZone && saved.day === currentDay
          ? saved
          : { timeZone, day: currentDay },
      );
    };
    const scheduleNextDay = () => {
      const now = Temporal.Now.instant();
      const today = now.toZonedDateTimeISO(timeZone).toPlainDate();
      const nextMidnight = today
        .add({ days: 1 })
        .toZonedDateTime({ timeZone })
        .toInstant();
      const delay = Math.max(
        1,
        Number(nextMidnight.epochMilliseconds - now.epochMilliseconds) + 1,
      );
      timeout = window.setTimeout(() => {
        refreshDay();
        scheduleNextDay();
      }, delay);
    };
    const reschedule = () => {
      refreshDay();
      window.clearTimeout(timeout);
      scheduleNextDay();
    };
    const refreshWhenVisible = () => {
      if (document.visibilityState === "visible") reschedule();
    };

    reschedule();
    window.addEventListener("focus", reschedule);
    document.addEventListener("visibilitychange", refreshWhenVisible);

    return () => {
      window.clearTimeout(timeout);
      window.removeEventListener("focus", reschedule);
      document.removeEventListener("visibilitychange", refreshWhenVisible);
    };
  }, [timeZone]);

  return day;
}

function currentLocalDay(timeZone: string) {
  return Temporal.Now.instant()
    .toZonedDateTimeISO(timeZone)
    .toPlainDate()
    .toString();
}

function chartRangeBounds(range: string, timeZone: string, day: string) {
  const today = Temporal.PlainDate.from(day);
  const endOfDay = today
    .add({ days: 1 })
    .toZonedDateTime({ timeZone })
    .subtract({ microseconds: 1 });
  const from =
    range === "All"
      ? undefined
      : endOfDay
          .subtract(
            range === "1Y"
              ? { years: 1 }
              : { months: Number.parseInt(range, 10) },
          )
          .toInstant()
          .toString();
  return { from, to: endOfDay.toInstant().toString() };
}

export function useWeightMutation() {
  const client = useQueryClient();
  const queryKey = weightEntriesQueryKey;

  return useMutation({
    mutationKey,
    mutationFn: ({ kind, entry }: WeightChange) =>
      api<WeightEntry | undefined>(
        kind === "add"
          ? "/api/weight-entries"
          : `/api/weight-entries/${encodeURIComponent(entry.id)}`,
        {
          method: { add: "POST", update: "PATCH", delete: "DELETE" }[kind],
          headers: { "content-type": "application/json" },
          body:
            kind === "delete"
              ? undefined
              : JSON.stringify({
                  weight_kg: entry.weight_kg,
                  measured_at: entry.measured_at,
                }),
        },
      ),
    onMutate: async ({ kind, entry }) => {
      await client.cancelQueries({ queryKey });
      const previous = client
        .getQueryData<WeightEntry[]>(queryKey)
        ?.find((item) => item.id === entry.id);
      client.setQueryData<WeightEntry[]>(queryKey, (entries) =>
        recentEntries([
          ...(entries ?? []).filter((item) => item.id !== entry.id),
          ...(kind === "delete" ? [] : [entry]),
        ]),
      );
      return { previous };
    },
    onError: (_error, { entry }, context) => {
      client.setQueryData<WeightEntry[]>(queryKey, (entries) =>
        recentEntries([
          ...(entries ?? []).filter((item) => item.id !== entry.id),
          ...(context?.previous ? [context.previous] : []),
        ]),
      );
    },
    onSuccess: (saved, { entry }) => {
      client.setQueryData<WeightEntry[]>(queryKey, (entries) =>
        recentEntries([
          ...(entries ?? []).filter((item) => item.id !== entry.id),
          ...(saved ? [saved] : []),
        ]),
      );
    },
    onSettled: () => {
      if (client.isMutating({ mutationKey }) === 1) {
        void client.invalidateQueries({ queryKey });
        void client.invalidateQueries({ queryKey: weightChartQueryKey });
      }
    },
  });
}
