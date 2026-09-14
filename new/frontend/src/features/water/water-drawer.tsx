import { Drawer } from "@base-ui/react/drawer";
import { Popover } from "@base-ui/react/popover";
import type { ReactNode } from "react";

import { useIsDesktop } from "../../lib/use-is-desktop.ts";
import {
  desktopWaterPopoverHandle,
  mobileWaterDrawerHandle,
} from "./water-drawer-handle.ts";
import { WaterEntryList } from "./water-entry-list.tsx";
import { WaterForm } from "./water-form.tsx";
import { WaterGoal } from "./water-goal.tsx";

export function WaterDrawer(props: { date: string }) {
  const isDesktop = useIsDesktop();

  return isDesktop ? (
    <DesktopWaterPopover date={props.date} />
  ) : (
    <MobileWaterDrawer date={props.date} />
  );
}

function MobileWaterDrawer(props: { date: string }) {
  return (
    <Drawer.Root handle={mobileWaterDrawerHandle} swipeDirection="right">
      <Drawer.Portal>
        <Drawer.Backdrop className="fixed inset-0 z-[100000] bg-black/10 opacity-[calc(1-var(--drawer-swipe-progress))] transition-opacity duration-[450ms] ease-[cubic-bezier(0.32,0.72,0,1)] data-[ending-style]:opacity-0 data-[ending-style]:[transition-duration:calc(var(--drawer-swipe-strength)*400ms)] data-[starting-style]:opacity-0 data-[swiping]:duration-0 motion-reduce:transition-none" />
        <Drawer.Viewport className="pointer-events-none fixed inset-0 z-[100001] flex justify-end">
          <Drawer.Popup className="pointer-events-auto h-full w-full max-w-md translate-x-[var(--drawer-swipe-movement-x)] overflow-hidden bg-canvas shadow-2xl outline-none transition-transform duration-[450ms] ease-[cubic-bezier(0.32,0.72,0,1)] will-change-transform data-[ending-style]:translate-x-full data-[ending-style]:[transition-duration:calc(var(--drawer-swipe-strength)*400ms)] data-[starting-style]:translate-x-full data-[swiping]:select-none motion-reduce:transition-none">
            <Drawer.Content className="h-full">
              <WaterPanel
                isMobile
                date={props.date}
                paddingClassName="px-4 pt-[max(1rem,env(safe-area-inset-top))]"
                title={<Drawer.Title className="sr-only">Water</Drawer.Title>}
                description={
                  <Drawer.Description className="sr-only">
                    Add water and review entries for this day.
                  </Drawer.Description>
                }
                close={
                  <Drawer.Close
                    aria-label="Close water log"
                    className="sr-only"
                  />
                }
              />
            </Drawer.Content>
          </Drawer.Popup>
        </Drawer.Viewport>
      </Drawer.Portal>
    </Drawer.Root>
  );
}

function DesktopWaterPopover(props: { date: string }) {
  return (
    <Popover.Root handle={desktopWaterPopoverHandle} modal>
      <Popover.Portal>
        <Popover.Backdrop className="fixed inset-0 z-[100000] bg-black/10 opacity-100 transition-opacity duration-150 ease-out data-[ending-style]:opacity-0 data-[starting-style]:opacity-0 motion-reduce:transition-none" />
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
          <Popover.Popup className="relative h-[min(44rem,calc(100dvh-2rem))] w-full origin-[var(--transform-origin)] overflow-hidden rounded-3xl border border-gray-200 bg-canvas shadow-2xl outline-none transition-[scale,opacity] duration-150 ease-out data-[ending-style]:scale-[0.98] data-[ending-style]:opacity-0 data-[starting-style]:scale-[0.98] data-[starting-style]:opacity-0 motion-reduce:transition-none">
            <WaterPanel
              date={props.date}
              paddingClassName="p-6"
              title={<Popover.Title className="sr-only">Water</Popover.Title>}
              description={
                <Popover.Description className="sr-only">
                  Add water and review entries for this day.
                </Popover.Description>
              }
              close={
                <Popover.Close
                  aria-label="Close water log"
                  className="absolute top-4 right-4 grid size-9 cursor-pointer place-items-center rounded-full text-gray-700 outline-2 outline-transparent outline-offset-2 hover:bg-gray-150 focus-visible:outline-gray-500"
                >
                  <CloseIcon />
                </Popover.Close>
              }
            />
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  );
}

function WaterPanel(props: {
  date: string;
  isMobile?: boolean;
  paddingClassName: string;
  title: ReactNode;
  description: ReactNode;
  close: ReactNode;
}) {
  return (
    <div className="h-full overflow-hidden [container-type:size]">
      <div className={`flex h-full min-h-0 flex-col ${props.paddingClassName}`}>
        <header className="flex items-center justify-between gap-4">
          {props.title}
          {props.close}
        </header>
        {props.description}
        {props.isMobile && <WaterGoal date={props.date} />}
        <WaterForm date={props.date} isMobile={props.isMobile} />
        <WaterEntryList date={props.date} />
      </div>
    </div>
  );
}

function CloseIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      className="size-5"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
    >
      <path d="m6 6 12 12M18 6 6 18" />
    </svg>
  );
}
