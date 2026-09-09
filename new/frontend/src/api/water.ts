import {
  keepPreviousData,
  type QueryClient,
  useMutation,
  useMutationState,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

import { api } from "./api.ts";
import { goalsProgressKey, type GoalsWithProgress } from "./goals.ts";

export type WaterEntry = {
  id: string;
  amount_ml: number;
  consumed_at: string;
};

type DeleteWaterInput = Pick<WaterEntry, "id" | "amount_ml">;

function changeWaterTotal(client: QueryClient, date: string, change: number) {
  client.setQueryData<GoalsWithProgress>(goalsProgressKey(date), (goals) =>
    goals
      ? {
          ...goals,
          progress: {
            ...goals.progress,
            water_ml: goals.progress.water_ml + change,
          },
        }
      : goals,
  );
}

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
    filters: { mutationKey: ["water-entry", date, "add"], status: "pending" },
    select: (mutation) => ({
      id: null,
      amount_ml: mutation.state.variables as number,
      consumed_at: new Date(mutation.state.submittedAt).toISOString(),
      submittedAt: mutation.state.submittedAt,
    }),
  });

  const deletingIds = useMutationState({
    filters: {
      mutationKey: ["water-entry", date, "delete"],
      status: "pending",
    },
    select: (mutation) => (mutation.state.variables as DeleteWaterInput).id,
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
  const progressKey = goalsProgressKey(date);

  return useMutation({
    mutationKey: ["water-entry", date, "add"],
    mutationFn: (amount_ml: number) =>
      api<WaterEntry>("/api/water-entries", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ amount_ml, date }),
      }),
    onMutate: async (amount) => {
      await Promise.all([
        queryClient.cancelQueries({ queryKey }),
        queryClient.cancelQueries({ queryKey: progressKey }),
      ]);
      changeWaterTotal(queryClient, date, amount);
    },
    onError: (_error, amount) => {
      changeWaterTotal(queryClient, date, -amount);
    },
    onSuccess: (savedEntry) => {
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) => [
        savedEntry,
        ...(entries ?? []).filter((entry) => entry.id !== savedEntry.id),
      ]);
    },
    onSettled: () => {
      if (
        queryClient.isMutating({ mutationKey: ["water-entry", date] }) === 1
      ) {
        void queryClient.invalidateQueries({ queryKey });
        void queryClient.invalidateQueries({ queryKey: ["goals", date] });
      }
    },
  });
}

export function useDeleteWaterMutation(date: string) {
  const queryClient = useQueryClient();
  const queryKey = waterEntriesQueryKey(date);
  const progressKey = goalsProgressKey(date);

  return useMutation({
    mutationKey: ["water-entry", date, "delete"],
    mutationFn: async (entry: DeleteWaterInput) => {
      await api<void>(`/api/water-entries/${encodeURIComponent(entry.id)}`, {
        method: "DELETE",
      });
      return entry;
    },
    onMutate: async (entry) => {
      await Promise.all([
        queryClient.cancelQueries({ queryKey }),
        queryClient.cancelQueries({ queryKey: progressKey }),
      ]);
      changeWaterTotal(queryClient, date, -entry.amount_ml);
    },
    onError: (_error, entry) => {
      changeWaterTotal(queryClient, date, entry.amount_ml);
    },
    onSuccess: (entry) => {
      queryClient.setQueryData<WaterEntry[]>(queryKey, (entries) =>
        entries?.filter((item) => item.id !== entry.id),
      );
    },
    onSettled: () => {
      if (
        queryClient.isMutating({ mutationKey: ["water-entry", date] }) === 1
      ) {
        void queryClient.invalidateQueries({ queryKey });
        void queryClient.invalidateQueries({ queryKey: ["goals", date] });
      }
    },
  });
}
