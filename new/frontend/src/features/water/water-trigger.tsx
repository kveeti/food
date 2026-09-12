import { Drawer } from "@base-ui/react/drawer";
import { Popover } from "@base-ui/react/popover";
import type { ButtonHTMLAttributes } from "react";

import { useIsDesktop } from "../../lib/use-is-desktop.ts";
import {
  desktopWaterPopoverHandle,
  mobileWaterDrawerHandle,
} from "./water-drawer-handle.ts";

export function WaterTrigger(props: ButtonHTMLAttributes<HTMLButtonElement>) {
  const isDesktop = useIsDesktop();

  return isDesktop ? (
    <Popover.Trigger {...props} handle={desktopWaterPopoverHandle} />
  ) : (
    <Drawer.Trigger {...props} handle={mobileWaterDrawerHandle} />
  );
}
