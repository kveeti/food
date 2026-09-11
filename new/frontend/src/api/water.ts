import {
  keepPreviousData,
  replaceEqualDeep,
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

type CachedWaterEntry = WaterEntry & { renderKey?: string };
type AddWaterInput = {
  amount_ml: number;
  renderKey: string;
};
type DeleteWaterInput = Pick<WaterEntry, "id" | "amount_ml">;

export type WaterEntryView =
  | (WaterEntry & { renderKey: string; isDeleting: boolean })
  | (Omit<WaterEntry, "id"> & {
      id: null;
      renderKey: string;
      isDeleting: false;
    });

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
  const query = useQuery<CachedWaterEntry[]>({
    queryKey: waterEntriesQueryKey(date),
    queryFn: ({ signal }) =>
      api<WaterEntry[]>(`/api/water-entries?date=${encodeURIComponent(date)}`, {
        signal,
      }),
    placeholderData: keepPreviousData,
    structuralSharing: (oldData, newData) =>
      keepRenderKeys(
        oldData as CachedWaterEntry[] | undefined,
        newData as CachedWaterEntry[],
      ),
  });
  const pendingEntries = useMutationState({
    filters: { mutationKey: ["water-entry", date, "add"], status: "pending" },
    select: (mutation) => {
      const input = mutation.state.variables as AddWaterInput;
      return {
        id: null,
        amount_ml: input.amount_ml,
        consumed_at: new Date(mutation.state.submittedAt).toISOString(),
        renderKey: input.renderKey,
        isDeleting: false,
      } satisfies WaterEntryView;
    },
  });

  const deletingIds = useMutationState({
    filters: {
      mutationKey: ["water-entry", date, "delete"],
      status: "pending",
    },
    select: (mutation) => (mutation.state.variables as DeleteWaterInput).id,
  });

  const entries: WaterEntryView[] = (query.data ?? []).map((entry) => ({
    ...entry,
    renderKey: entry.renderKey ?? entry.id,
    isDeleting: deletingIds.includes(entry.id),
  }));
  for (const pending of pendingEntries) {
    if (!entries.some((entry) => entry.renderKey === pending.renderKey)) {
      entries.unshift(pending);
    }
  }

  return { ...query, entries };
}

function keepRenderKeys(
  oldEntries: CachedWaterEntry[] | undefined,
  newEntries: CachedWaterEntry[],
) {
  const renderKeys = new Map(
    (oldEntries ?? []).map((entry) => [entry.id, entry.renderKey]),
  );
  return replaceEqualDeep(
    oldEntries,
    newEntries.map((entry) => ({
      ...entry,
      renderKey: renderKeys.get(entry.id) ?? entry.renderKey,
    })),
  ) as CachedWaterEntry[];
}

export function useAddWaterMutation(date: string) {
  const queryClient = useQueryClient();
  const queryKey = waterEntriesQueryKey(date);
  const progressKey = goalsProgressKey(date);

  return useMutation({
    mutationKey: ["water-entry", date, "add"],
    mutationFn: (input: AddWaterInput) =>
      api<WaterEntry>("/api/water-entries", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ amount_ml: input.amount_ml, date }),
      }),
    onMutate: async (input) => {
      await Promise.all([
        queryClient.cancelQueries({ queryKey }),
        queryClient.cancelQueries({ queryKey: progressKey }),
      ]);
      changeWaterTotal(queryClient, date, input.amount_ml);
    },
    onError: (_error, input) => {
      changeWaterTotal(queryClient, date, -input.amount_ml);
    },
    onSuccess: (savedEntry, input) => {
      queryClient.setQueryData<CachedWaterEntry[]>(queryKey, (entries) => [
        { ...savedEntry, renderKey: input.renderKey },
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
