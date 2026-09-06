import type { ReactNode } from "react";
import { Link, useLocation } from "wouter";

export function ImmediateNavLink(props: {
  href: string;
  className?: string;
  "aria-label"?: string;
  children: ReactNode;
}) {
  const [, navigate] = useLocation();
  const isLocal = () =>
    new URL(props.href, globalThis.location.href).origin ===
    globalThis.location.origin;
  const isPlain = (event: {
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    shiftKey: boolean;
  }) =>
    isLocal() &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey;

  return (
    <Link
      href={props.href}
      className={props.className}
      aria-label={props["aria-label"]}
      onClick={(event) => {
        if (event.button === 0 && isPlain(event)) event.preventDefault();
      }}
      onMouseDown={(event) => {
        if (event.button !== 0 || !isPlain(event)) return;
        event.preventDefault();
        navigate(props.href);
      }}
      onTouchStart={(event) => {
        if (!isPlain(event)) return;
        event.preventDefault();
        navigate(props.href);
      }}
      onKeyUp={(event) => {
        if (
          !isPlain(event) ||
          (event.key !== "Enter" && event.key !== " " && event.key !== "Space")
        ) {
          return;
        }
        event.preventDefault();
        navigate(props.href);
      }}
    >
      {props.children}
    </Link>
  );
}
