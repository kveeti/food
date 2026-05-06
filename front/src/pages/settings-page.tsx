import { useMutation, useQueryClient } from '@tanstack/solid-query';
import { Show, type Component } from 'solid-js';

import { api, useSettings, type Settings } from '../app.data';
import { PushSettings } from '../components/push-settings';
import { Field } from '../components/ui';

export const SettingsPage: Component = () => {
  const settings = useSettings();
  const queryClient = useQueryClient();
  const browserTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

  const saveSettings = useMutation(() => ({
    mutationFn: (settings: {
      mealIntervalMinutes: number;
      reminderOffsetMinutes: number;
      meals: string[];
      timezone: string;
    }) =>
      api<Settings>({
        path: '/api/v1/settings',
        method: 'PUT',
        body: settings,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['settings'] });
      queryClient.invalidateQueries({ queryKey: ['today'] });
      queryClient.invalidateQueries({ queryKey: ['meals'] });
    },
  }));

  const save = (event: Event) => {
    event.preventDefault();
    if (saveSettings.isPending) return;

    const formData = new FormData(event.currentTarget as HTMLFormElement);
    const intervalHours = Number(formValue(formData, 'mealIntervalHours'));

    saveSettings.mutate({
      mealIntervalMinutes: Math.round(intervalHours * 60),
      reminderOffsetMinutes: Number(formValue(formData, 'reminderOffsetMinutes')),
      meals: formValue(formData, 'meals')
        .split(',')
        .map((meal) => meal.trim())
        .filter(Boolean),
      timezone: formValue(formData, 'timezone') || browserTimezone,
    });
  };

  const currentSettings = () => settings.data ?? null;
  const mealIntervalHours = () => {
    const value = currentSettings()?.mealIntervalMinutes;
    return value ? (value / 60).toString() : '';
  };

  return (
    <div class="space-y-4">
      <h1 class="text-2xl leading-8 font-semibold tracking-[-0.005em]">
        Settings
      </h1>

      <div class="space-y-12">

        <form class="grid gap-4 sm:grid-cols-2" onSubmit={save}>
          <Field label="Meals">
            <input
              class="input"
              name="meals"
              placeholder="breakfast, lunch, dinner"
              value={currentSettings()?.meals.join(', ') ?? ''}
            />
          </Field>
          <Field label="Hours between meals (h)">
            <input
              class="input"
              name="mealIntervalHours"
              inputMode="decimal"
              min="0.1"
              placeholder="3.5"
              step="0.1"
              type="number"
              value={mealIntervalHours()}
            />
          </Field>
          <Field label="Reminder minutes before (m)">
            <input
              class="input"
              name="reminderOffsetMinutes"
              inputMode="numeric"
              min="0"
              placeholder="10"
              type="number"
              value={currentSettings()?.reminderOffsetMinutes.toString() ?? ''}
            />
          </Field>
          <Field label="Timezone">
            <input
              class="input"
              name="timezone"
              placeholder={browserTimezone}
              value={currentSettings()?.timezone ?? ''}
            />
          </Field>

          <Show when={saveSettings.error}>
            {(error) => (
              <p class="border-red-a6 bg-red-a4 text-red-12 border p-3 text-sm sm:col-span-2">
                {error().message}
              </p>
            )}
          </Show>

          <div class="sm:col-span-2">
            <button class="button" type="submit">
              {saveSettings.isPending ? 'Saving' : 'Save settings'}
            </button>
          </div>
        </form>

        <PushSettings settingsSet={currentSettings() !== null} />
      </div>
    </div>
  );
};

function formValue(formData: FormData, name: string): string {
  const value = formData.get(name);
  return typeof value === 'string' ? value : '';
}
