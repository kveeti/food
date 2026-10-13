import type { ReactNode } from "react";

export function Field(props: {
  label: string;
  error?: string;
  children: ReactNode;
}) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-sm text-gray-700">{props.label}</span>
      {props.children}
      {props.error && (
        <span className="text-sm text-[var(--input-invalid-text)]">
          {props.error}
        </span>
      )}
    </label>
  );
}
