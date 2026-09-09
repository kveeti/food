import {
  keepPreviousData,
  replaceEqualDeep,
  useMutation,
  useMutationState,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

import { api } from "./api.ts";

export type MealChoice =
  | "continue_previous"
  | "breakfast"
  | "lunch"
  | "snack"
  | "dinner";

export function useMealSuggestionQuery(date: string) {
  return useQuery({
    queryKey: ["meal-suggestion", date],
    queryFn: ({ signal }) =>
      api<{
        meal: MealChoice | null;
        previous_meal_id: string | null;
        previous_meal_name: string | null;
      }>(`/api/meal-suggestion?date=${date}`, { signal }),
    staleTime: 0,
  });
}

export type Nutrient = {
  code: string;
  name: string;
  unit: string;
  value: number;
  show_by_default: boolean;
};

export type Food = {
  id: string;
  display_name: string;
  brand: string | null;
  basis_unit: "g" | "ml";
  source: string | null;
};
export type FoodDetail = Food & { nutrients: Nutrient[] };
export type FoodEntry = {
  id: string;
  meal_id: string | null;
  meal_name: string | null;
  food_id: string | null;
  food_name: string;
  food_brand: string | null;
  amount: number;
  unit: "g" | "ml";
  eaten_at: string;
  nutrients: Nutrient[];
};
export type FoodMeal = {
  id: string | null;
  name: string | null;
  started_at: string;
  entries: FoodEntry[];
};

type CachedFoodEntry = FoodEntry & { renderKey?: string };
type CachedFoodMeal = Omit<FoodMeal, "entries"> & {
  renderKey?: string;
  entries: CachedFoodEntry[];
};

type AddFoodInput = Omit<FoodEntry, "id"> & {
  meal: MealChoice;
  renderKey: string;
};

export type FoodEntryView =
  | (FoodEntry & {
      renderKey: string;
    })
  | (Omit<FoodEntry, "id"> & {
      id: null;
      renderKey: string;
    });

export type FoodMealView = Omit<FoodMeal, "entries"> & {
  renderKey: string;
  entries: FoodEntryView[];
};

export function useFoodSearchQuery(query: string) {
  return useQuery({
    queryKey: ["food-search", query],
    queryFn: ({ signal }) =>
      api<Food[]>(`/api/foods?q=${encodeURIComponent(query)}`, { signal }),
    enabled: query.length > 0,
    placeholderData: keepPreviousData,
    staleTime: 60_000,
  });
}

export function useFoodQuery(id: string) {
  return useQuery({
    queryKey: ["food", id],
    queryFn: ({ signal }) =>
      api<FoodDetail>(`/api/foods/${encodeURIComponent(id)}`, { signal }),
  });
}

const mealsKey = (date: string) => ["meals", date] as const;

export function useFoodMealsQuery(date: string) {
  const query = useQuery<CachedFoodMeal[]>({
    queryKey: mealsKey(date),
    queryFn: ({ signal }) =>
      api<FoodMeal[]>(`/api/meals?date=${date}`, { signal }),
    structuralSharing: (oldData, newData) =>
      keepRenderKeys(
        oldData as CachedFoodMeal[] | undefined,
        newData as CachedFoodMeal[],
      ),
  });
  const pendingEntries = useMutationState({
    filters: { mutationKey: ["food-entry", date, "add"], status: "pending" },
    select: (mutation) => ({
      ...(mutation.state.variables as AddFoodInput),
      id: null,
    }),
  });

  const meals: FoodMealView[] = (query.data ?? []).map((meal) => ({
    ...meal,
    renderKey: meal.renderKey ?? meal.id ?? meal.entries[0]?.id ?? "meal",
    entries: meal.entries.map((entry) => ({
      ...entry,
      renderKey: entry.renderKey ?? entry.id,
    })),
  }));

  for (const pending of pendingEntries) {
    if (
      meals.some((meal) =>
        meal.entries.some((entry) => entry.renderKey === pending.renderKey),
      )
    ) {
      continue;
    }
    const existingMeal = pending.meal_id
      ? meals.findIndex((meal) => meal.id === pending.meal_id)
      : -1;
    if (existingMeal >= 0) {
      const meal = meals[existingMeal]!;
      meals[existingMeal] = {
        ...meal,
        entries: [pending, ...meal.entries],
      };
    } else {
      meals.unshift({
        id: pending.meal_id,
        name: pending.meal_name,
        started_at: pending.eaten_at,
        renderKey: pending.meal_id ?? `meal-${pending.renderKey}`,
        entries: [pending],
      });
    }
  }

  return { ...query, meals };
}

function keepRenderKeys(
  oldMeals: CachedFoodMeal[] | undefined,
  newMeals: CachedFoodMeal[],
) {
  const entryKeys = new Map<string, string>();
  const mealKeys = new Map<string, string>();
  for (const meal of oldMeals ?? []) {
    if (meal.id && meal.renderKey) mealKeys.set(meal.id, meal.renderKey);
    for (const entry of meal.entries) {
      if (entry.renderKey) entryKeys.set(entry.id, entry.renderKey);
    }
  }
  const merged = newMeals.map((meal) => ({
    ...meal,
    renderKey: (meal.id && mealKeys.get(meal.id)) || meal.renderKey,
    entries: meal.entries.map((entry) => ({
      ...entry,
      renderKey: entryKeys.get(entry.id) ?? entry.renderKey,
    })),
  }));
  return replaceEqualDeep(oldMeals, merged) as CachedFoodMeal[];
}

function removeEntry(meals: CachedFoodMeal[], id: string) {
  return meals
    .map((meal) => ({
      ...meal,
      entries: meal.entries.filter((entry) => entry.id !== id),
    }))
    .filter((meal) => meal.entries.length > 0);
}

function replaceEntry(meals: CachedFoodMeal[], entry: FoodEntry) {
  return meals.map((meal) => ({
    ...meal,
    entries: meal.entries.map((item) =>
      item.id === entry.id ? { ...entry, renderKey: item.renderKey } : item,
    ),
  }));
}

export function useAddFoodMutation(date: string) {
  const client = useQueryClient();
  const queryKey = mealsKey(date);
  return useMutation({
    mutationKey: ["food-entry", date, "add"],
    mutationFn: (entry: AddFoodInput) =>
      api<FoodEntry>("/api/food-entries", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          meal: entry.meal,
          meal_id: entry.meal_id,
          food_id: entry.food_id,
          amount: entry.amount,
          unit: entry.unit,
          date,
        }),
      }),
    onMutate: () => client.cancelQueries({ queryKey }),
    onSuccess: (entry, input) => {
      client.setQueryData<CachedFoodMeal[]>(queryKey, (meals = []) => {
        const savedEntry = { ...entry, renderKey: input.renderKey };
        const existingMeal = meals.findIndex(
          (meal) => meal.id === entry.meal_id,
        );
        if (existingMeal >= 0) {
          return meals.map((meal, index) =>
            index === existingMeal
              ? {
                  ...meal,
                  entries: [
                    savedEntry,
                    ...meal.entries.filter((item) => item.id !== entry.id),
                  ],
                }
              : meal,
          );
        }
        return [
          {
            id: entry.meal_id,
            name: entry.meal_name,
            started_at: entry.eaten_at,
            renderKey:
              input.meal === "continue_previous"
                ? (entry.meal_id ?? undefined)
                : `meal-${input.renderKey}`,
            entries: [savedEntry],
          },
          ...meals,
        ];
      });
    },
    onSettled: () => {
      void client.invalidateQueries({ queryKey: ["meal-suggestion", date] });
      if (client.isMutating({ mutationKey: ["food-entry", date] }) === 1) {
        void client.invalidateQueries({ queryKey });
        void client.invalidateQueries({ queryKey: ["goals", date] });
      }
    },
  });
}

export function useDeleteFoodMutation(date: string) {
  const client = useQueryClient();
  const queryKey = mealsKey(date);
  return useMutation({
    mutationKey: ["food-entry", date, "delete"],
    mutationFn: (entryId: string) =>
      api<void>(`/api/food-entries/${entryId}`, { method: "DELETE" }),
    onMutate: () => client.cancelQueries({ queryKey }),
    onSuccess: (_data, entryId) => {
      client.setQueryData<CachedFoodMeal[]>(queryKey, (meals = []) =>
        removeEntry(meals, entryId),
      );
    },
    onSettled: () => {
      void client.invalidateQueries({ queryKey: ["meal-suggestion", date] });
      if (client.isMutating({ mutationKey: ["food-entry", date] }) === 1) {
        void client.invalidateQueries({ queryKey });
        void client.invalidateQueries({ queryKey: ["goals", date] });
      }
    },
  });
}

export function useUpdateFoodMutation(date: string) {
  const client = useQueryClient();
  const queryKey = mealsKey(date);
  return useMutation({
    mutationKey: ["food-entry", date, "update"],
    mutationFn: ({ entry, amount }: { entry: FoodEntry; amount: number }) =>
      api<FoodEntry>(`/api/food-entries/${entry.id}`, {
        method: "PATCH",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ amount }),
      }),
    onMutate: async ({ entry, amount }) => {
      await client.cancelQueries({ queryKey });
      client.setQueryData<CachedFoodMeal[]>(queryKey, (meals = []) =>
        replaceEntry(meals, {
          ...entry,
          amount,
          nutrients: entry.nutrients.map((n) => ({
            ...n,
            value: (n.value * amount) / entry.amount,
          })),
        }),
      );
    },
    onError: (_error, { entry }) => {
      client.setQueryData<CachedFoodMeal[]>(queryKey, (meals = []) =>
        replaceEntry(meals, entry),
      );
    },
    onSuccess: (entry) => {
      client.setQueryData<CachedFoodMeal[]>(queryKey, (meals = []) =>
        replaceEntry(meals, entry),
      );
    },
    onSettled: () => {
      if (client.isMutating({ mutationKey: ["food-entry", date] }) === 1) {
        void client.invalidateQueries({ queryKey });
        void client.invalidateQueries({ queryKey: ["goals", date] });
      }
    },
  });
}
