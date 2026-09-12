import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import {
  type KeyboardEvent,
  type PointerEvent,
  useLayoutEffect,
  useReducer,
  useRef,
} from "react";

import { useAddWaterMutation } from "../../api/water.ts";
import { createId } from "../../lib/id.ts";
import { Button } from "../../ui/button/button.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

const STEP = 10;
const VESSELS = {
  glass: {
    label: "Glass",
    min: 10,
    max: 600,
    defaultAmount: 250,
    top: 10,
    bottom: 220,
    className: "h-[280px] w-[180px]",
    viewBox: "0 0 140 230",
    path: "M10 10 H130 L115 207 Q114 220 100 220 H40 Q26 220 25 207 Z",
  },
  bottle: {
    label: "Bottle",
    min: 100,
    max: 1500,
    defaultAmount: 1000,
    top: 24,
    bottom: 232,
    className: "h-[310px] w-[190px]",
    viewBox: "0 0 140 240",
    path: "M58 6 H82 Q88 6 88 12 V18 Q88 22 86 22 Q88 24 92 24 C114 24 132 46 132 72 V204 Q132 230 106 230 H34 Q8 230 8 204 V72 C8 46 26 24 48 24 Q52 24 54 22 Q52 22 52 18 V12 Q52 6 58 6 Z",
  },
} as const;

type VesselName = keyof typeof VESSELS;
type WaterState = {
  selected: VesselName;
  amounts: Record<VesselName, number>;
};
type WaterAction =
  | { type: "select"; vessel: VesselName }
  | { type: "setAmount"; vessel: VesselName; amount: number };

const initialState: WaterState = {
  selected: "glass",
  amounts: {
    glass: VESSELS.glass.defaultAmount,
    bottle: VESSELS.bottle.defaultAmount,
  },
};

function waterReducer(state: WaterState, action: WaterAction): WaterState {
  if (action.type === "select") {
    return { ...state, selected: action.vessel };
  }
  return {
    ...state,
    amounts: { ...state.amounts, [action.vessel]: action.amount },
  };
}

export function WaterForm(props: { date: string }) {
  const addWater = useAddWaterMutation(props.date);
  const [state, dispatch] = useReducer(waterReducer, initialState);
  const amount = state.amounts[state.selected];
  const { f } = useI18n();
  const reducedMotion = useReducedMotion();

  return (
    <form
      className="mt-6 flex flex-col items-center"
      onSubmit={(event) => {
        event.preventDefault();
        if (!addWater.isPending) {
          addWater.mutate({
            amount_ml: amount,
            renderKey: createId(),
          });
        }
      }}
    >
      <div className="relative h-[420px] w-48">
        {(Object.keys(VESSELS) as VesselName[]).map((name) => (
          <WaterVessel
            key={name}
            name={name}
            amount={state.amounts[name]}
            selected={state.selected === name}
            onSelect={() => dispatch({ type: "select", vessel: name })}
            onAmount={(nextAmount) =>
              dispatch({
                type: "setAmount",
                vessel: name,
                amount: nextAmount,
              })
            }
          />
        ))}
      </div>

      <output className="mt-3 text-[2rem] font-semibold tabular-nums text-gray-950">
        {f.number(amount)} ml
      </output>

      <div className="mt-5 w-full max-w-xs">
        <AnimatePresence initial={false}>
          {addWater.isError && (
            <motion.p
              role="alert"
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{
                duration: reducedMotion ? 0 : 0.2,
                ease: [0.16, 1, 0.3, 1],
              }}
              className="overflow-hidden"
            >
              <span className="mb-2 block rounded-lg bg-danger-surface px-2 py-1 text-sm text-danger-fg">
                Error adding water
              </span>
            </motion.p>
          )}
        </AnimatePresence>
        <Button type="submit" className="w-full">
          {addWater.isPending ? "Adding…" : "Add water"}
        </Button>
      </div>
    </form>
  );
}

function WaterVessel(props: {
  name: VesselName;
  amount: number;
  selected: boolean;
  onSelect: () => void;
  onAmount: (amount: number) => void;
}) {
  const { f } = useI18n();
  const vessel = VESSELS[props.name];
  const setAmount = (value: number) => {
    props.onAmount(
      Math.min(
        vessel.max,
        Math.max(vessel.min, Math.round(value / STEP) * STEP),
      ),
    );
  };
  const setFromPointer = (
    event: PointerEvent<HTMLDivElement>,
    capture: boolean,
  ) => {
    if (!props.selected) return;
    if (capture) event.currentTarget.setPointerCapture(event.pointerId);
    if (!capture && !event.currentTarget.hasPointerCapture(event.pointerId)) {
      return;
    }
    event.preventDefault();
    const bounds = event.currentTarget.getBoundingClientRect();
    const ratio = (bounds.bottom - event.clientY) / bounds.height;
    setAmount(ratio * vessel.max);
  };
  const liquidY =
    vessel.bottom -
    (props.amount / vessel.max + (1 - props.amount / vessel.max) * 0.025) *
      (vessel.bottom - vessel.top);

  return (
    <div
      role="slider"
      tabIndex={0}
      data-selected={props.selected}
      data-base-ui-swipe-ignore={props.selected || undefined}
      aria-label={`${vessel.label} amount`}
      aria-current={props.selected}
      aria-valuemin={vessel.min}
      aria-valuemax={vessel.max}
      aria-valuenow={props.amount}
      aria-valuetext={`${f.number(props.amount)} millilitres`}
      className={`water-vessel rounded-2xl outline-2 outline-transparent outline-offset-4 focus-visible:outline-gray-500 ${vessel.className}`}
      onClick={props.onSelect}
      onPointerDown={(event) => setFromPointer(event, true)}
      onPointerMove={(event) => setFromPointer(event, false)}
      onKeyDown={(event) =>
        handleVesselKey(
          event,
          props.selected,
          vessel,
          props.onSelect,
          setAmount,
        )
      }
    >
      <VesselGraphic name={props.name} liquidY={liquidY} />
    </div>
  );
}

function handleVesselKey(
  event: KeyboardEvent<HTMLDivElement>,
  selected: boolean,
  vessel: (typeof VESSELS)[VesselName],
  select: () => void,
  setAmount: (amount: number) => void,
) {
  if (!selected) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      select();
    }
    return;
  }

  if (event.key === "Enter") {
    event.preventDefault();
    event.currentTarget.closest("form")?.requestSubmit();
    return;
  }

  const amount = Number(event.currentTarget.getAttribute("aria-valuenow"));
  let next: number | undefined;
  if (event.key === "ArrowUp" || event.key === "ArrowRight") {
    next = amount + STEP;
  }
  if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
    next = amount - STEP;
  }
  if (event.key === "Home") next = vessel.min;
  if (event.key === "End") next = vessel.max;
  if (next === undefined) return;
  event.preventDefault();
  setAmount(next);
}

function VesselGraphic(props: { name: VesselName; liquidY: number }) {
  const vessel = VESSELS[props.name];
  const waterClip = `${props.name}-water-clip`;
  const bubbleClip = `${props.name}-bubble-clip`;
  const liquidRef = useRef<SVGGElement>(null);
  const bubbleClipRef = useRef<SVGRectElement>(null);
  const currentY = useRef(props.liquidY);
  const reducedMotion = useReducedMotion();

  useLayoutEffect(() => {
    let frame: number | undefined;
    let previousTime: number | undefined;
    const targetY = props.liquidY;

    function renderLiquid() {
      const transform = `translateY(${currentY.current}px)`;
      if (liquidRef.current) liquidRef.current.style.transform = transform;
      if (bubbleClipRef.current)
        bubbleClipRef.current.style.transform = transform;
    }

    function animate(time: number) {
      const elapsed = Math.min(time - (previousTime ?? time - 16), 32);
      const distance = targetY - currentY.current;
      if (Math.abs(distance) < 0.05) {
        currentY.current = targetY;
        renderLiquid();
        return;
      }
      currentY.current += distance * (1 - Math.exp(-elapsed / 64));
      renderLiquid();
      previousTime = time;
      frame = requestAnimationFrame(animate);
    }

    renderLiquid();
    if (reducedMotion) {
      currentY.current = targetY;
      renderLiquid();
    } else if (currentY.current !== targetY) {
      frame = requestAnimationFrame(animate);
    }

    return () => {
      if (frame !== undefined) cancelAnimationFrame(frame);
    };
  }, [props.liquidY, reducedMotion]);

  return (
    <svg
      viewBox={vessel.viewBox}
      preserveAspectRatio="none"
      className="h-full w-full"
      aria-hidden="true"
    >
      <defs>
        <clipPath id={waterClip}>
          <path d={vessel.path} />
        </clipPath>
        <clipPath id={bubbleClip}>
          <rect
            ref={bubbleClipRef}
            className="water-bubble-clip"
            x="0"
            y="8"
            width="140"
            height="292"
          />
        </clipPath>
      </defs>
      <path className="water-vessel-outline" d={vessel.path} />
      <g clipPath={`url(#${waterClip})`}>
        <g ref={liquidRef} className="water-liquid">
          <path
            className="water-wave water-wave-back"
            d="M-192 4 C-168 2 -120 2 -96 4 C-72 6 -24 6 0 4 C24 2 72 2 96 4 C120 6 168 6 192 4 C216 2 264 2 288 4 C312 6 360 6 384 4 V300 H-192 Z"
          />
          <path
            className="water-wave water-wave-front"
            d="M-192 5 C-168 2 -120 2 -96 5 C-72 8 -24 8 0 5 C24 2 72 2 96 5 C120 8 168 8 192 5 C216 2 264 2 288 5 C312 8 360 8 384 5 V300 H-192 Z"
          />
        </g>
        <g clipPath={`url(#${bubbleClip})`}>
          <WaterBubbles />
        </g>
      </g>
    </svg>
  );
}

function WaterBubbles() {
  const bubbles = [
    ["one", 35, 2.5],
    ["two", 66, 2],
    ["three", 98, 3],
    ["four", 51, 2.5],
    ["five", 84, 1.75],
  ] as const;

  return (
    <g className="water-bubbles">
      {bubbles.map(([name, x, radius]) => (
        <g key={name} className={`water-bubble-path water-bubble-${name}`}>
          <circle className="water-bubble" cx={x} cy="0" r={radius} />
        </g>
      ))}
    </g>
  );
}
