import { useQuery } from '@tanstack/solid-query';

export type Settings = {
  mealIntervalMinutes: number;
  reminderOffsetMinutes: number;
  meals: string[];
  remindersPaused: boolean;
  timezone: string;
  updatedAt: string;
};

export type Meal = {
  id: number;
  mealType: string;
  startedAt: string;
  endedAt: string | null;
  note: string | null;
  createdAt: string;
  updatedAt: string;
};

export type Today = {
  settingsRequired: boolean;
  settings: Settings | null;
  currentMeal: Meal | null;
  meals: Meal[];
  mealsStartedToday: number;
  remindersPaused: boolean;
  nextMeal: {
    dueAt: string;
    reminderAt: string;
    alreadySentReminder: boolean;
  } | null;
};

export type MealHistory = {
  days: {
    date: string;
    meals: Meal[];
  }[];
};

export type PushPublicKey = {
  publicKey: string;
  configured: boolean;
};

export type BrowserPushState = {
  supported: boolean;
  permission: NotificationPermission | 'unsupported';
  subscribed: boolean;
};

type ApiProps = {
  path: string;
  method?: string;
  body?: unknown;
  signal?: AbortSignal;
};

export async function api<TReturnValue>(props: ApiProps): Promise<TReturnValue> {
  const fetchProps: RequestInit = {
    signal: props.signal,
    method: props.method ?? 'GET',
  };

  if (props.body) {
    fetchProps.body = JSON.stringify(props.body);
    fetchProps.headers = { 'Content-Type': 'application/json' };
  }

  const response = await fetch(props.path, fetchProps).catch(() => {
    throw new Error('network error');
  });
  const json = await response.json().catch(() => null);

  if (!response.ok) {
    throw new Error(json?.error ?? `unexpected server error - status: ${response.status}`);
  }

  return json as TReturnValue;
}

export function settingsQueryOptions() {
  return {
    queryKey: ['settings'],
    queryFn: ({ signal }: { signal?: AbortSignal }) =>
      api<Settings | null>({
        path: '/api/v1/settings',
        signal,
      }),
  };
}

export function useSettings() {
  return useQuery(() => settingsQueryOptions());
}

export function todayQueryOptions() {
  return {
    queryKey: ['today'],
    queryFn: ({ signal }: { signal?: AbortSignal }) =>
      api<Today>({
        path: '/api/v1/today',
        signal,
      }),
  };
}

export function mealsQueryOptions() {
  return {
    queryKey: ['meals'],
    queryFn: ({ signal }: { signal?: AbortSignal }) =>
      api<MealHistory>({
        path: '/api/v1/meals?limit=100',
        signal,
      }),
  };
}

export function pushPublicKeyQueryOptions() {
  return {
    queryKey: ['push', 'public-key'],
    queryFn: ({ signal }: { signal?: AbortSignal }) =>
      api<PushPublicKey>({
        path: '/api/v1/push/public-key',
        signal,
      }),
  };
}

export function browserPushStateQueryOptions() {
  return {
    queryKey: ['push', 'browser-state'],
    queryFn: getBrowserPushState,
  };
}

async function getBrowserPushState(): Promise<BrowserPushState> {
  if (!('serviceWorker' in navigator) || !('PushManager' in window)) {
    return {
      supported: false,
      permission: 'unsupported',
      subscribed: false,
    };
  }

  const registration = await navigator.serviceWorker.getRegistration();
  const subscription = await registration?.pushManager.getSubscription();

  return {
    supported: true,
    permission: Notification.permission,
    subscribed: Boolean(subscription),
  };
}
