import { Field as FormField, Form, useForm } from "@formisch/react";
import * as v from "valibot";
import { useLocation } from "wouter";

import { useSaveSettingsMutation, type User } from "../../api/user.ts";
import { Button } from "../../ui/button/button.tsx";
import { Field } from "../../ui/input/field.tsx";
import { Input } from "../../ui/input/input.tsx";
import { I18n } from "../i18n/i18n.tsx";
import { GoalSettings } from "./goal-settings.tsx";

const schema = v.object({
  locale: v.pipe(v.string(), v.trim(), v.nonEmpty("Enter a locale")),
  timezone: v.pipe(v.string(), v.trim(), v.nonEmpty("Enter a timezone")),
});

const timezones = Intl.supportedValuesOf?.("timeZone") ?? [];

export default function SettingsPage(props: { user: User }) {
  const [, navigate] = useLocation();
  const mutation = useSaveSettingsMutation();
  const needsSetup = !props.user.locale || !props.user.timezone;
  const form = useForm({
    schema,
    initialInput: {
      locale: props.user.locale ?? navigator.language,
      timezone:
        props.user.timezone ??
        Intl.DateTimeFormat().resolvedOptions().timeZone ??
        "",
    },
  });

  return (
    <main className="mx-auto w-full max-w-[var(--page-width)] px-4 py-10 sm:px-7">
      <header className="mb-8">
        <h1 className="text-xl font-semibold tracking-tight text-gray-950">
          {needsSetup ? "Set up Food" : "Settings"}
        </h1>
        {needsSetup && (
          <p className="mt-2 text-base text-gray-700">
            Choose your locale and timezone to continue.
          </p>
        )}
      </header>

      <Form
        of={form}
        className="flex flex-col gap-5"
        onSubmit={async (settings) => {
          if (mutation.isPending) return;
          try {
            await mutation.mutateAsync(settings);
            navigate("/");
          } catch {
            // The mutation error is shown below.
          }
        }}
      >
        <FormField of={form} path={["locale"]}>
          {(field) => (
            <Field label="Locale" error={field.errors?.[0]}>
              <Input
                {...field.props}
                error={!!field.errors}
                value={field.input ?? ""}
                placeholder="en-FI"
                autoComplete="language"
              />
            </Field>
          )}
        </FormField>

        <FormField of={form} path={["timezone"]}>
          {(field) => (
            <Field label="Timezone" error={field.errors?.[0]}>
              <Input
                {...field.props}
                error={!!field.errors}
                value={field.input ?? ""}
                placeholder="Europe/Helsinki"
                autoComplete="off"
                list="timezones"
              />
              <datalist id="timezones">
                {timezones.map((timezone) => (
                  <option key={timezone} value={timezone} />
                ))}
              </datalist>
            </Field>
          )}
        </FormField>

        {mutation.isError && (
          <p className="text-base text-danger-fg">{mutation.error.message}</p>
        )}

        <div className="mt-2 flex justify-end">
          <Button type="submit">
            {mutation.isPending ? "Saving…" : "Save"}
          </Button>
        </div>
      </Form>
      {!needsSetup && (
        <I18n locale={props.user.locale!} timeZone={props.user.timezone!}>
          <GoalSettings />
        </I18n>
      )}
    </main>
  );
}
