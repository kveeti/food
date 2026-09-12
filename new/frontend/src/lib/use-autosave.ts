import { type FormSchema, type FormStore, validate } from "@formisch/react";
import { useCallback, useEffect, useRef } from "react";
import type * as v from "valibot";

export type SaveStatus = "idle" | "saving" | "saved" | "error";

export function useAutosave<TSchema extends FormSchema>(options: {
  form: FormStore<TSchema>;
  save: (values: v.InferOutput<TSchema>) => Promise<unknown>;
  onStatusChange: (status: SaveStatus) => void;
  delay?: number;
}) {
  const timeout = useRef<ReturnType<typeof setTimeout>>(undefined);
  const isSaving = useRef(false);
  const hasChanges = useRef(false);
  const isReady = useRef(false);
  const isMounted = useRef(false);
  const optionsRef = useRef(options);
  const runRef = useRef<() => void>(() => undefined);

  useEffect(() => {
    optionsRef.current = options;
  }, [options]);

  const run = useCallback(() => {
    if (isSaving.current || !hasChanges.current || !isReady.current) return;

    isSaving.current = true;
    hasChanges.current = false;
    isReady.current = false;

    void (async () => {
      try {
        const result = await validate(optionsRef.current.form);
        if (!result.success) {
          if (isMounted.current && !hasChanges.current) {
            optionsRef.current.onStatusChange("idle");
          }
          return;
        }

        if (isMounted.current) {
          optionsRef.current.onStatusChange("saving");
        }
        await optionsRef.current.save(result.output);
        if (isMounted.current) {
          if (!hasChanges.current) {
            optionsRef.current.onStatusChange("saved");
          } else if (!isReady.current) {
            optionsRef.current.onStatusChange("idle");
          }
        }
      } catch {
        if (isMounted.current) {
          if (!hasChanges.current) {
            optionsRef.current.onStatusChange("error");
          } else if (!isReady.current) {
            optionsRef.current.onStatusChange("idle");
          }
        }
      } finally {
        isSaving.current = false;
        runRef.current();
      }
    })();
  }, []);

  useEffect(() => {
    runRef.current = run;
  }, [run]);

  const flush = useCallback(() => {
    if (timeout.current) clearTimeout(timeout.current);
    isReady.current = true;
    run();
  }, [run]);

  const schedule = useCallback(() => {
    if (timeout.current) clearTimeout(timeout.current);
    hasChanges.current = true;
    isReady.current = false;
    optionsRef.current.onStatusChange("idle");
    timeout.current = setTimeout(flush, optionsRef.current.delay ?? 600);
  }, [flush]);

  useEffect(() => {
    isMounted.current = true;
    return () => {
      isMounted.current = false;
      flush();
    };
  }, [flush]);

  return { flush, schedule };
}
