import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { useLocation, useSearch } from "wouter";

import { useAddFoodMutation } from "../../api/food.ts";
import { SelectedFood } from "./food-form.tsx";

export function NewFoodSection(props: { date: string; onClose: () => void }) {
  const search = useSearch();
  const [, navigate] = useLocation();
  const reducedMotion = useReducedMotion();
  const params = new URLSearchParams(search);
  const food = params.get("food");
  const mutation = useAddFoodMutation(props.date);
  const submitting = mutation.isPending && mutation.variables?.food_id === food;
  const close = (focusSearch: boolean) => {
    const current = new URLSearchParams(window.location.search);
    if (
      current.get("food") !== food ||
      current.get("date") !== params.get("date")
    )
      return;
    mutation.reset();
    current.delete("food");
    navigate(`/${current.size ? `?${current}` : ""}`, { replace: true });
    if (focusSearch) props.onClose();
  };

  return (
    <AnimatePresence initial={false}>
      {food && (
        <motion.section
          aria-label="New food"
          aria-hidden={submitting || undefined}
          inert={submitting}
          className="overflow-hidden"
          initial={{ opacity: 0, height: "auto" }}
          animate={{
            opacity: submitting ? 0 : 1,
            height: submitting ? 0 : "auto",
          }}
          exit={{ opacity: 0, height: 0 }}
          transition={{
            height: {
              duration: reducedMotion ? 0 : 0.18,
              ease: [0.16, 1, 0.3, 1],
            },
            opacity: {
              duration: reducedMotion ? 0 : 0.3,
              ease: [0.16, 1, 0.3, 1],
            },
          }}
        >
          <SelectedFood
            key={`${props.date}:${food}`}
            id={food}
            date={props.date}
            mutation={mutation}
            onSaved={() => close(false)}
            onClose={() => close(true)}
          />
        </motion.section>
      )}
    </AnimatePresence>
  );
}
