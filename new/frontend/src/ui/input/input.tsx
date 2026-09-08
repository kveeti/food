import { forwardRef, type InputHTMLAttributes } from "react";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "size"> & {
  error?: boolean;
  size?: "default" | "small";
};

export const Input = forwardRef<HTMLInputElement, Props>(function Input(
  { error, className, size, ...props },
  ref,
) {
  const _size = size ?? "default";
  return (
    <input
      {...props}
      ref={ref}
      aria-invalid={error ? true : undefined}
      className={`w-full rounded-xl border border-transparent bg-[var(--input-bg)] px-3 font-[inherit] text-gray-1000 outline-2 outline-transparent outline-offset-[-1px] placeholder:text-[var(--input-placeholder)] hover:bg-[var(--input-bg-alt)] focus-visible:outline-[var(--input-ring-active)] ${
        error ? "outline-[var(--input-invalid-ring)]" : ""
      } ${_size === "small" ? "h-9" : "h-10"} ${className ?? ""}`}
    />
  );
});
