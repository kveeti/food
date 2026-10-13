import { forwardRef, type InputHTMLAttributes } from "react";

import styles from "./date-input.module.css";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "size" | "type">;

export const DateInput = forwardRef<HTMLInputElement, Props>(function DateInput(
  { className, ...props },
  ref,
) {
  return (
    <span className="block h-10 w-full min-w-0 overflow-hidden rounded-xl bg-[var(--input-bg)] outline-2 outline-transparent outline-offset-[-1px] hover:bg-[var(--input-bg-alt)] has-[input:focus-visible]:outline-[var(--input-ring-active)]">
      <input
        {...props}
        ref={ref}
        type="date"
        className={`${styles.input} block h-full w-full min-w-0 max-w-full border-0 bg-transparent font-[inherit] text-gray-1000 outline-none ${className ?? ""}`}
      />
    </span>
  );
});
