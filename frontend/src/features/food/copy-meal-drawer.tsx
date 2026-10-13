import { Drawer } from "@base-ui/react/drawer";
import { Popover } from "@base-ui/react/popover";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { type FoodMealView, useCopyMealMutation } from "../../api/food.ts";
import { useIsDesktop } from "../../lib/use-is-desktop.ts";
import { Button } from "../../ui/button/button.tsx";
import { CopyMealForm } from "./copy-meal-form.tsx";

export function CopyMealDrawer(props: {
  meal: FoodMealView;
  selectedDate: string;
}) {
  const isDesktop = useIsDesktop();
  const [isOpen, setIsOpen] = useState(false);
  const copy = useCopyMealMutation();
  const form = (
    <CopyMealForm
      meal={props.meal}
      selectedDate={props.selectedDate}
      mutation={copy}
      onClose={() => setIsOpen(false)}
    />
  );
  const trigger = (
    <Button type="button" variant="ghost">
      Copy
    </Button>
  );

  function changeOpen(
    isOpen: boolean,
    event: Drawer.Root.ChangeEventDetails | Popover.Root.ChangeEventDetails,
  ) {
    if (copy.isPending) {
      event.cancel();
      return;
    }
    if (isOpen) copy.reset();
    setIsOpen(isOpen);
  }

  return isDesktop ? (
    <Popover.Root open={isOpen} onOpenChange={changeOpen} modal>
      <Popover.Trigger render={trigger} />
      <Popover.Portal>
        <Popover.Backdrop className="fixed inset-0 z-[100000] bg-black/10 transition-opacity duration-150 ease-out data-[ending-style]:opacity-0 data-[starting-style]:opacity-0 motion-reduce:transition-none" />
        <Popover.Positioner
          side="right"
          align="start"
          sideOffset={12}
          positionMethod="fixed"
          collisionPadding={16}
          collisionAvoidance={{
            side: "shift",
            align: "shift",
            fallbackAxisSide: "none",
          }}
          className="z-[100001] w-[min(28rem,calc(100vw-2rem))]"
        >
          <Popover.Popup className="max-h-[min(44rem,calc(100dvh-2rem))] w-full origin-[var(--transform-origin)] overflow-hidden rounded-3xl border border-gray-200 bg-canvas shadow-2xl outline-none transition-[scale,opacity] duration-150 ease-out data-[ending-style]:scale-[0.98] data-[ending-style]:opacity-0 data-[starting-style]:scale-[0.98] data-[starting-style]:opacity-0 motion-reduce:transition-none">
            <CopyMealPanel
              isSaving={copy.isPending}
              paddingClassName="p-6"
              title={
                <Popover.Title className="text-lg font-semibold text-gray-950">
                  Copy meal
                </Popover.Title>
              }
              description={
                <Popover.Description className="sr-only">
                  Choose foods, amounts, and when to log them.
                </Popover.Description>
              }
              close={
                <Popover.Close
                  render={
                    <Button variant="ghost" aria-label="Close meal copy">
                      Close
                    </Button>
                  }
                />
              }
            >
              {form}
            </CopyMealPanel>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  ) : (
    <Drawer.Root open={isOpen} onOpenChange={changeOpen} swipeDirection="right">
      <Drawer.Trigger render={trigger} />
      <Drawer.Portal>
        <Drawer.Backdrop className="fixed inset-0 z-[100000] bg-black/10 opacity-[calc(1-var(--drawer-swipe-progress))] transition-opacity duration-[450ms] ease-[cubic-bezier(0.32,0.72,0,1)] data-[ending-style]:opacity-0 data-[ending-style]:[transition-duration:calc(var(--drawer-swipe-strength)*400ms)] data-[starting-style]:opacity-0 data-[swiping]:duration-0 motion-reduce:transition-none" />
        <Drawer.Viewport className="pointer-events-none fixed inset-0 z-[100001] flex justify-end">
          <Drawer.Popup className="pointer-events-auto h-dvh w-full max-w-md translate-x-[var(--drawer-swipe-movement-x)] overflow-hidden bg-canvas shadow-2xl outline-none transition-transform duration-[450ms] ease-[cubic-bezier(0.32,0.72,0,1)] will-change-transform data-[ending-style]:translate-x-full data-[ending-style]:[transition-duration:calc(var(--drawer-swipe-strength)*400ms)] data-[starting-style]:translate-x-full data-[swiping]:select-none motion-reduce:transition-none">
            <Drawer.Content className="h-full">
              <CopyMealPanel
                isSaving={copy.isPending}
                paddingClassName="px-4 pt-[max(1rem,env(safe-area-inset-top))] pb-[max(1rem,env(safe-area-inset-bottom))]"
                title={
                  <Drawer.Title className="text-lg font-semibold text-gray-950">
                    Copy meal
                  </Drawer.Title>
                }
                description={
                  <Drawer.Description className="sr-only">
                    Choose foods, amounts, and when to log them.
                  </Drawer.Description>
                }
                close={
                  <Drawer.Close
                    render={
                      <Button variant="ghost" aria-label="Close meal copy">
                        Close
                      </Button>
                    }
                  />
                }
              >
                {form}
              </CopyMealPanel>
            </Drawer.Content>
          </Drawer.Popup>
        </Drawer.Viewport>
      </Drawer.Portal>
    </Drawer.Root>
  );
}

function CopyMealPanel(props: {
  isSaving: boolean;
  paddingClassName: string;
  title: ReactNode;
  description: ReactNode;
  close: ReactNode;
  children: ReactNode;
}) {
  const savingStatus = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (props.isSaving) savingStatus.current?.focus({ preventScroll: true });
  }, [props.isSaving]);

  return (
    <div
      className="relative h-full max-h-[inherit]"
      data-base-ui-swipe-ignore={props.isSaving || undefined}
    >
      <div
        inert={props.isSaving}
        aria-hidden={props.isSaving || undefined}
        aria-busy={props.isSaving}
        className={`h-full max-h-[inherit] overflow-y-auto overscroll-contain transition-[filter,opacity] duration-150 motion-reduce:transition-none ${props.isSaving ? "blur-[2px] opacity-50" : ""}`}
      >
        <div className={props.paddingClassName}>
          <header className="mb-5 flex items-center justify-between gap-3">
            {props.title}
            {props.close}
          </header>
          {props.description}
          {props.children}
        </div>
      </div>
      {props.isSaving && (
        <div
          ref={savingStatus}
          role="status"
          tabIndex={-1}
          className="absolute inset-0 grid place-items-center outline-none"
        >
          <span className="px-4 py-2 text-lg font-medium text-gray-950">
            Copying...
          </span>
        </div>
      )}
    </div>
  );
}
