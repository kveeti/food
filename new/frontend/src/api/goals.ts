import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

import { api } from "./api.ts";

export type NutrientGoal = {
  code: string;
  name: string;
  unit: string;
  goal: number | null;
  show_by_default: boolean;
};

export type Goals = {
  starts_on: string | null;
  daily_burn_kcal: number | null;
  food_adjustment_kcal: number | null;
  calorie_goal_kcal: number | null;
  water_ml: number | null;
  nutrients: NutrientGoal[];
};

export type GoalsWithProgress = Goals & {
  progress: {
    calories: {
      eaten: number;
      incomplete: boolean;
      unknown: boolean;
    };
    water_ml: number;
    nutrients: {
      code: string;
      eaten: number;
      incomplete: boolean;
      unknown: boolean;
    }[];
  };
};

export type SaveGoals = {
  starts_on: string;
  daily_burn_kcal: number | null;
  food_adjustment_kcal: number | null;
  water_ml: number | null;
  nutrients: { code: string; value: number }[];
};

const goalsKey = (date: string) => ["goals", date] as const;
export const goalsProgressKey = (date: string) =>
  [...goalsKey(date), "progress"] as const;

export function useGoalsQuery(date: string) {
  return useQuery({
    queryKey: goalsKey(date),
    queryFn: ({ signal }) =>
      api<Goals>(`/api/goals?date=${encodeURIComponent(date)}`, { signal }),
    enabled: /^\d{4}-\d{2}-\d{2}$/.test(date),
  });
}

export function useGoalsProgressQuery(date: string) {
  return useQuery({
    queryKey: goalsProgressKey(date),
    queryFn: ({ signal }) =>
      api<GoalsWithProgress>(
        `/api/goals?date=${encodeURIComponent(date)}&include=progress`,
        { signal },
      ),
    enabled: /^\d{4}-\d{2}-\d{2}$/.test(date),
    placeholderData: keepPreviousData,
  });
}

export function useSaveGoalsMutation() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: (goals: SaveGoals) =>
      api<void>("/api/goals", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(goals),
      }),
    onSettled: () => {
      void client.invalidateQueries({ queryKey: ["goals"] });
    },
  });
}
