import { type InputHTMLAttributes, forwardRef } from "react";

type Props = InputHTMLAttributes<HTMLInputElement> & {
  error?: string;
};

export const Input = forwardRef<HTMLInputElement, Props>(function Input(
  { error, className, ...props },
  ref,
) {
  return (
    <input
      {...props}
      ref={ref}
      aria-invalid={error ? true : undefined}
      className={`h-10 w-full rounded-xl border border-transparent bg-[var(--input-bg)] px-3 font-[inherit] text-gray-1000 outline-2 outline-transparent outline-offset-[-1px] placeholder:text-[var(--input-placeholder)] hover:bg-[var(--input-bg-alt)] focus-visible:outline-[var(--input-ring-active)] ${error ? "outline-[var(--input-invalid-ring)]" : ""} ${className ?? ""}`}
    />
  );
});
