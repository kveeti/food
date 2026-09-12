import { Field as FormField, Form, useForm } from "@formisch/react";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";
import * as v from "valibot";
import { useLocation } from "wouter";

import { useSaveSettingsMutation, type User } from "../../api/user.ts";
import { type SaveStatus, useAutosave } from "../../lib/use-autosave.ts";
import { useIsReducedMotion } from "../../lib/use-is-reduced-motion.ts";
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
  const [profileSaveStatus, setProfileSaveStatus] =
    useState<SaveStatus>("idle");
  const [goalSaveStatus, setGoalSaveStatus] = useState<SaveStatus>("idle");
  const saveStatus = combinedSaveStatus(profileSaveStatus, goalSaveStatus);
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
  const autosave = useAutosave({
    form,
    save: (settings) => mutation.mutateAsync(settings),
    onStatusChange: setProfileSaveStatus,
  });

  useEffect(() => {
    if (saveStatus !== "saved") return;
    const timeout = setTimeout(() => {
      setProfileSaveStatus((status) => (status === "saved" ? "idle" : status));
      setGoalSaveStatus((status) => (status === "saved" ? "idle" : status));
    }, 2_000);
    return () => clearTimeout(timeout);
  }, [saveStatus]);

  return (
    <main className="flex min-h-0 min-w-0 w-full flex-1 flex-col overflow-hidden">
      <header className="z-[5] shrink-0 bg-canvas/90 backdrop-blur-md">
        <div className="mx-auto w-full max-w-[var(--page-max-width)] px-[var(--page-padding)] pt-[max(0.75rem,env(safe-area-inset-top,0px))] pb-3">
          <div className="flex items-center justify-between gap-3">
            <h1 className="text-xl font-semibold tracking-tight text-gray-950">
              {needsSetup ? "Set up Food" : "You"}
            </h1>
            <SaveIndicator status={saveStatus} />
          </div>
          {needsSetup && (
            <p className="mt-2 text-base text-gray-700">
              Choose your locale and timezone to continue.
            </p>
          )}
        </div>
      </header>

      <div
        className="min-h-0 flex-1 overflow-y-auto overscroll-contain sm:[scrollbar-gutter:stable_both-edges]"
        data-settings-scroll
      >
        <div className="mx-auto w-full max-w-[var(--page-max-width)] px-[var(--page-padding)] pt-8 pb-[calc(var(--nav-clearance)+2.5rem)]">
          <Form
            of={form}
            className="flex min-w-0 flex-col gap-5"
            onChange={() => {
              if (!needsSetup) autosave.schedule();
            }}
            onBlur={(event) => {
              if (
                !needsSetup &&
                !event.currentTarget.contains(event.relatedTarget)
              ) {
                autosave.flush();
              }
            }}
            onSubmit={async (settings) => {
              if (!needsSetup) {
                autosave.flush();
                return;
              }
              if (mutation.isPending) return;
              setProfileSaveStatus("saving");
              try {
                await mutation.mutateAsync(settings);
                setProfileSaveStatus("saved");
                navigate("/");
              } catch {
                setProfileSaveStatus("error");
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
              <p role="alert" className="text-base text-danger-fg">
                Error saving settings
              </p>
            )}

            {needsSetup && (
              <div className="mt-2 flex justify-end">
                <Button type="submit">
                  {mutation.isPending ? "Saving…" : "Save"}
                </Button>
              </div>
            )}
          </Form>
          {!needsSetup && (
            <I18n locale={props.user.locale!} timeZone={props.user.timezone!}>
              <GoalSettings onSaveStatusChange={setGoalSaveStatus} />
            </I18n>
          )}
          <form method="post" action="/logout" className="mt-10">
            <Button type="submit" variant="outline">
              Log out
            </Button>
          </form>
        </div>
      </div>
    </main>
  );
}

function combinedSaveStatus(...statuses: SaveStatus[]): SaveStatus {
  if (statuses.includes("saving")) return "saving";
  if (statuses.includes("error")) return "error";
  if (statuses.includes("saved")) return "saved";
  return "idle";
}

function SaveIndicator(props: { status: SaveStatus }) {
  const isReducedMotion = useIsReducedMotion();
  const label =
    props.status === "saving"
      ? "Saving settings"
      : props.status === "saved"
        ? "Settings saved"
        : props.status === "error"
          ? "Settings not saved"
          : "";

  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className="grid h-8 min-w-8 shrink-0 place-items-center"
    >
      {props.status !== "saved" && <span className="sr-only">{label}</span>}
      <AnimatePresence initial={false} mode="popLayout">
        {props.status !== "idle" && (
          <motion.span
            key={props.status}
            aria-hidden={props.status === "saved" ? undefined : true}
            className="col-start-1 row-start-1 flex h-6 items-center justify-center gap-1.5 text-sm text-gray-700"
            initial={{
              opacity: 0,
              scale: isReducedMotion ? 1 : 0.7,
              filter: isReducedMotion ? "blur(0px)" : "blur(3px)",
            }}
            animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
            exit={{
              opacity: 0,
              scale: isReducedMotion ? 1 : 0.8,
              filter: isReducedMotion ? "blur(0px)" : "blur(3px)",
            }}
            transition={{
              duration: isReducedMotion ? 0 : 0.2,
              ease: [0.16, 1, 0.3, 1],
            }}
          >
            {props.status === "saving" ? (
              <span className="size-4 animate-spin rounded-full border-2 border-gray-350 border-t-gray-900 motion-reduce:animate-none" />
            ) : props.status === "saved" ? (
              <>
                <CheckIcon />
                <span>Saved</span>
              </>
            ) : (
              <ErrorIcon />
            )}
          </motion.span>
        )}
      </AnimatePresence>
    </div>
  );
}

function CheckIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-5 text-success-fg">
      <path
        d="m5 12.5 4.25 4.25L19 7"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
    </svg>
  );
}

function ErrorIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-5 text-danger-fg">
      <circle
        cx="12"
        cy="12"
        r="8"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path
        d="M12 8v5m0 3v.01"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="2"
      />
    </svg>
  );
}
