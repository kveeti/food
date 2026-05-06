import { useMutation, useQuery, useQueryClient } from '@tanstack/solid-query';
import {
  For,
  Show,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  type Component,
} from 'solid-js';

import {
  api,
  mealsQueryOptions,
  todayQueryOptions,
  type Meal,
  type Today,
} from '../app.data';
import { ErrorBlock, Loading } from '../components/ui';
import { formatDate, formatDuration, formatTime } from '../lib/format';

export const TrackerPage: Component = () => {
  const queryClient = useQueryClient();
  const today = useQuery(() => ({
    ...todayQueryOptions(),
    refetchInterval: 30000,
  }));
  const history = useQuery(() => mealsQueryOptions());
  const [now, setNow] = createSignal(Date.now());

  const invalidateMealQueries = () => {
    queryClient.invalidateQueries({ queryKey: ['today'] });
    queryClient.invalidateQueries({ queryKey: ['meals'] });
  };

  const startMeal = useMutation(() => ({
    mutationFn: (mealType: string) =>
      api<Today>({
        path: '/api/v1/meals/start',
        method: 'POST',
        body: { mealType },
      }),
    onSuccess: invalidateMealQueries,
  }));

  const stopMeal = useMutation(() => ({
    mutationFn: () =>
      api<Today>({
        path: '/api/v1/meals/current/stop',
        method: 'POST',
      }),
    onSuccess: invalidateMealQueries,
  }));

  const deleteMeal = useMutation(() => ({
    mutationFn: (id: number) =>
      api<{ ok: boolean }>({
        path: `/api/v1/meals/${id}`,
        method: 'DELETE',
      }),
    onSuccess: invalidateMealQueries,
  }));

  const toggleReminders = useMutation(() => ({
    mutationFn: (paused: boolean) =>
      api<Today>({
        path: paused ? '/api/v1/reminders/resume' : '/api/v1/reminders/pause',
        method: 'POST',
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['settings'] });
      queryClient.invalidateQueries({ queryKey: ['today'] });
    },
  }));

  const startMealIfIdle = (mealType: string) => {
    if (startMeal.isPending) return;
    startMeal.mutate(mealType);
  };

  const stopMealIfIdle = () => {
    if (stopMeal.isPending) return;
    stopMeal.mutate();
  };

  const deleteMealIfIdle = (id: number) => {
    if (deleteMeal.isPending) return;
    deleteMeal.mutate(id);
  };

  const toggleRemindersIfIdle = () => {
    if (toggleReminders.isPending || today.data === undefined) return;
    toggleReminders.mutate(today.data.remindersPaused);
  };

  onMount(() => {
    const tick = window.setInterval(() => setNow(Date.now()), 1000);
    onCleanup(() => window.clearInterval(tick));
  });

  const nextMealText = createMemo(() => {
    const next = today.data?.nextMeal;
    if (today.data?.remindersPaused) return 'Paused';
    if (!next) return 'No meal scheduled';

    const remaining = new Date(next.dueAt).getTime() - now();
    if (remaining <= 0) return 'Due now';

    return `In ${formatDuration(remaining)}`;
  });

  const pageStatus = createMemo(() => {
    const currentMeal = today.data?.currentMeal;
    if (currentMeal) {
      return {
        title: currentMeal.mealType,
        detail: formatDuration(now() - new Date(currentMeal.startedAt).getTime()),
      };
    }

    const next = today.data?.nextMeal;
    if (today.data?.remindersPaused || !next) {
      return { title: 'Paused', detail: '' };
    }

    return {
      title: `Next at ${formatTime(next.dueAt)}`,
      detail: nextMealText(),
    };
  });

  const nextMealType = createMemo(() => {
    const settings = today.data?.settings;
    if (!settings?.meals.length) return '';

    const lastMealType = history.data?.days[0]?.meals[0]?.mealType;
    if (!lastMealType) {
      return settings.meals[0];
    }

    const lastMealTypeIndex = settings.meals.findIndex(
      (meal) => meal === lastMealType,
    );
    if (lastMealTypeIndex === -1) {
      return settings.meals[0];
    }

    const nextMealTypeIndex = (lastMealTypeIndex + 1) % settings.meals.length;
    return settings.meals[nextMealTypeIndex];
  });

  return (
    <div class="space-y-5">
      <Show when={!today.isLoading} fallback={<Loading label="Loading today" />}>
        <Show when={!today.isError} fallback={<ErrorBlock error={today.error} />}>
          <h1 class="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-2xl leading-8 font-semibold tracking-[-0.005em]">
            <span>{pageStatus().title}</span>
            <Show when={pageStatus().detail}>
              <span class="text-gray-11 text-sm leading-5 font-medium tracking-normal">
                {pageStatus().detail}
              </span>
            </Show>
          </h1>

          <Show
            when={today.data?.currentMeal}
            fallback={
              <section class="space-y-2">
                <StartMealForm
                  meals={today.data?.settings?.meals ?? []}
                  selectedMealType={nextMealType()}
                  isPending={startMeal.isPending}
                  onStart={startMealIfIdle}
                />
                <RemindersButton
                  paused={today.data?.remindersPaused ?? false}
                  onClick={toggleRemindersIfIdle}
                />
              </section>
            }
          >
            {(meal) => (
              <section class="space-y-2">
                <p class="text-gray-11 text-xs">
                  Started {formatTime(meal().startedAt)}
                </p>
                <div class="grid gap-2 sm:grid-cols-2">
                  <button
                    type="button"
                    class="button w-full"
                    onClick={stopMealIfIdle}
                  >
                    Stop meal
                  </button>
                  <RemindersButton
                    paused={today.data?.remindersPaused ?? false}
                    onClick={toggleRemindersIfIdle}
                  />
                </div>
              </section>
            )}
          </Show>

          <MealHistory
            days={history.data?.days ?? []}
            now={now()}
            deleteMeal={deleteMealIfIdle}
          />
        </Show>
      </Show>
    </div>
  );
};

const RemindersButton: Component<{
  paused: boolean;
  onClick: () => void;
}> = (props) => (
  <button type="button" class="button-secondary w-full" onClick={props.onClick}>
    {props.paused ? 'Resume reminders' : 'Pause reminders'}
  </button>
);

const MealHistory: Component<{
  days: Array<{ date: string; meals: Array<Meal> }>;
  now: number;
  deleteMeal: (id: number) => void;
}> = (props) => (
  <section class="space-y-3">
    <h2 class="text-lg leading-6 font-semibold tracking-[-0.005em]">
      Meals
    </h2>
    <div class="divide-gray-a3 divide-y">
      <Show
        when={props.days.length > 0}
        fallback={<p class="text-gray-a11 bg-gray-a2 px-4 py-5">No meals yet</p>}
      >
        <For each={props.days}>
          {(day) => (
            <div>
              <div class="bg-gray-3 text-gray-11 px-3 py-2 text-[12px] font-medium sticky top-8 z-10">
                {formatDate(day.date)} · {day.meals.length}{' '}
                {day.meals.length === 1 ? 'meal' : 'meals'}
              </div>
              <For each={day.meals}>
                {(meal) => (
                  <div class="grid grid-cols-[1fr_auto] items-center gap-3 px-3 py-2.5">
                    <div class="min-w-0">
                      <p class="text-sm leading-5 font-semibold">
                        {meal.mealType}
                      </p>
                      <p class="text-gray-11 text-xs leading-5">
                        {meal.endedAt
                          ? `${formatTime(meal.startedAt)} - ${formatTime(meal.endedAt)}`
                          : `${formatTime(meal.startedAt)} - in progress`}
                      </p>
                    </div>
                    <div class="flex items-center gap-2">
                      <p class="text-gray-11 min-w-12 text-right text-xs tabular-nums">
                        {meal.endedAt
                          ? formatDuration(
                              new Date(meal.endedAt).getTime() -
                                new Date(meal.startedAt).getTime(),
                            )
                          : formatDuration(props.now - new Date(meal.startedAt).getTime())}
                      </p>
                      <button
                        type="button"
                        class="button button-destructive h-8 px-2 text-[12px]"
                        onClick={() => props.deleteMeal(meal.id)}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </div>
          )}
        </For>
      </Show>
    </div>
  </section>
);

const StartMealForm: Component<{
  meals: string[];
  selectedMealType: string;
  isPending: boolean;
  onStart: (mealType: string) => void;
}> = (props) => {
  const selectableMealTypes = createMemo(() => {
    const seen = new Set<string>();
    return props.meals.filter((meal) => {
      if (seen.has(meal)) return false;
      seen.add(meal);
      return true;
    });
  });

  const start = (event: Event) => {
    event.preventDefault();
    if (props.isPending) return;

    const formData = new FormData(event.currentTarget as HTMLFormElement);
    const mealType = formData.get('mealType');

    if (typeof mealType === 'string' && mealType) {
      props.onStart(mealType);
    }
  };

  return (
    <Show when={selectableMealTypes().length > 0}>
      <form class="grid gap-2 sm:grid-cols-[1fr_auto]" onSubmit={start}>
        <select class="input" name="mealType" value={props.selectedMealType}>
          <For each={selectableMealTypes()}>
            {(mealType) => <option value={mealType}>{mealType}</option>}
          </For>
        </select>
        <button type="submit" class="button">
          Start meal
        </button>
      </form>
    </Show>
  );
};
