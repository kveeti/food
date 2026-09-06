import {
  keepPreviousData,
  useMutation,
  useMutationState,
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
  const query = useQuery({
    queryKey: waterEntriesQueryKey(date),
    queryFn: ({ signal }) =>
      api<WaterEntry[]>(`/api/water-entries?date=${encodeURIComponent(date)}`, {
        signal,
      }),
    placeholderData: keepPreviousData,
  });
  const pendingEntries = useMutationState({
    filters: { mutationKey: ["add-water", date], status: "pending" },
    select: (mutation) => ({
      id: null,
      amount_ml: mutation.state.variables as number,
      consumed_at: new Date(mutation.state.submittedAt).toISOString(),
      submittedAt: mutation.state.submittedAt,
    }),
  });

  const deletingIds = useMutationState({
    filters: { mutationKey: ["delete-water", date], status: "pending" },
    select: (mutation) => mutation.state.variables as string,
  });

  return {
    ...query,
    entries: [...pendingEntries, ...(query.data ?? [])].map((entry) => ({
      ...entry,
      isDeleting: entry.id !== null && deletingIds.includes(entry.id),
    })),
  };
}

export function useAddWaterMutation(date: string) {
  const queryClient = useQueryClient();
  const queryKey = waterEntriesQueryKey(date);

  return useMutation({
    mutationKey: ["add-water", date],
    mutationFn: (amount_ml: number) =>
      api<WaterEntry>("/api/water-entries", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ amount_ml, date }),
      }),
    onMutate: () => queryClient.cancelQueries({ queryKey }),
    onSuccess: (savedEntry) => {
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) => [
        savedEntry,
        ...(entries ?? []).filter((entry) => entry.id !== savedEntry.id),
      ]);
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });
}

export function useDeleteWaterMutation(date: string) {
  const queryClient = useQueryClient();
  const queryKey = waterEntriesQueryKey(date);

  return useMutation({
    mutationKey: ["delete-water", date],
    mutationFn: (entryId: string) =>
      api<void>(`/api/water-entries/${encodeURIComponent(entryId)}`, {
        method: "DELETE",
      }),
    onMutate: () => queryClient.cancelQueries({ queryKey }),
    onSuccess: (_data, entryId) => {
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) =>
        entries?.filter((entry) => entry.id !== entryId),
      );
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });
}
