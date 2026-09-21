import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "./api.ts";

export type WeightEntry = {
  id: string;
  weight_kg: number;
  measured_at: string;
};

type WeightChange = {
  kind: "add" | "update" | "delete";
  entry: WeightEntry;
};

const queryKey = ["weight-entries"] as const;
const mutationKey = ["weight-entry"] as const;

export function useWeightEntriesQuery() {
  const query = useQuery({
    queryKey,
    queryFn: ({ signal }) =>
      api<WeightEntry[]>("/api/weight-entries", { signal }),
  });

  return {
    ...query,
    entries: (query.data ?? []).toSorted(
      (a, b) =>
        Date.parse(b.measured_at) - Date.parse(a.measured_at) ||
        b.id.localeCompare(a.id),
    ),
  };
}

export function useWeightMutation() {
  const client = useQueryClient();

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
      client.setQueryData<WeightEntry[]>(queryKey, (entries) => [
        ...(entries ?? []).filter((item) => item.id !== entry.id),
        ...(kind === "delete" ? [] : [entry]),
      ]);
      return { previous };
    },
    onError: (_error, { entry }, context) => {
      client.setQueryData<WeightEntry[]>(queryKey, (entries) => [
        ...(entries ?? []).filter((item) => item.id !== entry.id),
        ...(context?.previous ? [context.previous] : []),
      ]);
    },
    onSuccess: (saved, { entry }) => {
      client.setQueryData<WeightEntry[]>(queryKey, (entries) => [
        ...(entries ?? []).filter((item) => item.id !== entry.id),
        ...(saved ? [saved] : []),
      ]);
    },
    onSettled: () => {
      if (client.isMutating({ mutationKey }) === 1) {
        void client.invalidateQueries({ queryKey });
      }
    },
  });
}
