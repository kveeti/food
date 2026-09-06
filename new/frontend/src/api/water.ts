import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

import { api } from "./api.ts";

export type WaterEntry = {
  id: string;
  amount_ml: number;
  consumed_at: string;
};

const waterEntriesQueryKey = (date: string) => ["water-entries", date] as const;

export function useWaterEntriesQuery(date: string) {
  return useQuery({
    queryKey: waterEntriesQueryKey(date),
    queryFn: () =>
      api<WaterEntry[]>(`/api/water-entries?date=${encodeURIComponent(date)}`),
    placeholderData: keepPreviousData,
  });
}

export function useAddWaterMutation(date: string) {
  const queryClient = useQueryClient();
  const queryKey = waterEntriesQueryKey(date);

  return useMutation({
    mutationFn: (entry: WaterEntry) =>
      api<WaterEntry>("/api/water-entries", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: entry.id,
          amount_ml: entry.amount_ml,
          date,
        }),
      }),
    onMutate: async (entry) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<WaterEntry[]>(queryKey);
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) => [
        entry,
        ...(entries ?? []),
      ]);
      return { previous };
    },
    onError: (_error, _entry, context) => {
      queryClient.setQueryData(queryKey, context?.previous);
      void queryClient.invalidateQueries({ queryKey });
    },
    onSuccess: (savedEntry) => {
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) =>
        (entries ?? []).map((entry) =>
          entry.id === savedEntry.id ? savedEntry : entry,
        ),
      );
    },
  });
}

export function useDeleteWaterMutation(date: string) {
  const queryClient = useQueryClient();
  const queryKey = waterEntriesQueryKey(date);

  return useMutation({
    mutationFn: (entryId: string) =>
      api<void>(`/api/water-entries/${encodeURIComponent(entryId)}`, {
        method: "DELETE",
      }),
    onMutate: async (entryId) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<WaterEntry[]>(queryKey);
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) =>
        entries?.filter((entry) => entry.id !== entryId),
      );
      return { previous };
    },
    onError: (_error, _entryId, context) => {
      queryClient.setQueryData(queryKey, context?.previous);
      void queryClient.invalidateQueries({ queryKey });
    },
  });
}
