import { useReducedMotion } from "framer-motion";

export function useIsReducedMotion() {
  const val = useReducedMotion();

  return val ?? false;
}
