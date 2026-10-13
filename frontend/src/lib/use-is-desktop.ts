import { useSyncExternalStore } from "react";

const query = "(min-width: 40rem)";

function subscribe(onChange: () => void) {
  const media = matchMedia(query);
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}

function getSnapshot() {
  return matchMedia(query).matches;
}

export function useIsDesktop() {
  return useSyncExternalStore(subscribe, getSnapshot, () => false);
}
