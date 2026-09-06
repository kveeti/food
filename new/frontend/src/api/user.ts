import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "./api.ts";

export type User = {
  id: string;
  email: string | null;
  locale: string | null;
  timezone: string | null;
};

export type UserSettings = {
  locale: string;
  timezone: string;
};

export const meQueryKey = ["me"] as const;

export function useMeQuery() {
  return useQuery({
    queryKey: meQueryKey,
    queryFn: () => api<User>("/api/me"),
    retry: false,
  });
}

export function useSaveSettingsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: UserSettings) =>
      api<UserSettings>("/api/settings", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(settings),
      }),
    onMutate: async (settings) => {
      await queryClient.cancelQueries({ queryKey: meQueryKey });
      const previous = queryClient.getQueryData<User>(meQueryKey);
      queryClient.setQueryData<User>(meQueryKey, (user) =>
        user ? { ...user, ...settings } : user,
      );
      return { previous };
    },
    onError: (_error, _settings, context) => {
      queryClient.setQueryData(meQueryKey, context?.previous);
    },
    onSuccess: (settings) => {
      queryClient.setQueryData<User>(meQueryKey, (user) =>
        user ? { ...user, ...settings } : user,
      );
    },
  });
}
