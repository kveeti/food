import { DropdownMenu, type DropdownMenuItemProps } from "@kobalte/core/dropdown-menu";
import type { PolymorphicProps } from "@kobalte/core/polymorphic";
import { A, createAsync, useAction, useNavigate } from "@solidjs/router";
import { Suspense, type ComponentProps, type ValidComponent } from "solid-js";
import { getCurrentUser, logout } from "../lib/user";
import { AccountIcon } from "./icons/account-icon";
import { LogoutIcon } from "./icons/logout-icon";
import { MenuIcon } from "./icons/menu-icon";

export function Nav() {
  const user = createAsync(() => getCurrentUser());
  const runLogout = useAction(logout);

  return (
    <nav class="fixed inset-x-0 bottom-0 z-10 h-[calc(2.75rem+env(safe-area-inset-bottom))] bg-zinc-100/80 pb-[env(safe-area-inset-bottom)] backdrop-blur-md sm:top-0 sm:bottom-auto sm:h-9 sm:pb-0">
      <div class="mx-auto flex size-full max-w-4xl justify-between sm:px-4 pwa:pr-[calc(0.5rem+var(--safe-area-right))] pwa:pl-[calc(0.5rem+var(--safe-area-left))] sm:pwa:px-4">
        <A
          class="inline-flex h-full items-center px-3.5 text-sm no-underline hover:bg-zinc-200/80 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-zinc-900 sm:px-2"
          activeClass="underline"
          end
          href="/"
        >
          home
        </A>
        <DropdownMenu placement="bottom-end" gutter={8}>
          <DropdownMenu.Trigger
            class="inline-flex h-full w-11 cursor-pointer items-center justify-center hover:bg-zinc-200/80 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-zinc-900 sm:w-auto sm:px-2"
            aria-label="Open menu"
          >
            <MenuIcon class="size-4" aria-hidden="true" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content class="z-20 min-w-56 dropdown-menu-animation border border-zinc-200/60 bg-white p-1 shadow-lg outline-none">
              <MenuItem as={ImmediateNavLink} href="/profile">
                <AccountIcon class="size-4 shrink-0" aria-hidden="true" />
                <Suspense fallback={<span class="text-zinc-500">Profile</span>}>
                  <span class="truncate">{user()?.email}</span>
                </Suspense>
              </MenuItem>
              <DropdownMenu.Separator class="-mx-1 my-1 h-px border-0 bg-zinc-200" />
              <MenuItem onSelect={() => void runLogout()}>
                <LogoutIcon class="size-4 shrink-0" aria-hidden="true" />
                Log out
              </MenuItem>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu>
      </div>
    </nav>
  );
}

function MenuItem<T extends ValidComponent = "div">(
  props: PolymorphicProps<T, DropdownMenuItemProps<T>>,
) {
  return (
    <DropdownMenu.Item
      {...props}
      class="flex cursor-pointer items-center gap-2 px-3 py-2 text-sm no-underline outline-none data-highlighted:bg-zinc-100"
    />
  );
}

function ImmediateNavLink(props: ComponentProps<typeof A>) {
  const navigate = useNavigate();
  const isLocal = () => new URL(props.href, location.href).origin === location.origin;
  const hasModifier = (event: MouseEvent | TouchEvent | KeyboardEvent) =>
    event.altKey || event.ctrlKey || event.metaKey || event.shiftKey;

  const go = (event: MouseEvent | TouchEvent | KeyboardEvent) => {
    if (!isLocal() || hasModifier(event)) return;
    event.preventDefault();
    navigate(props.href);
  };

  return (
    <A
      {...props}
      on:click={(event) => {
        if (isLocal() && event.button === 0 && !hasModifier(event)) event.preventDefault();
      }}
      on:mousedown={(event) => {
        if (event.button === 0) go(event);
      }}
      on:touchstart={go}
      on:keydown={(event) => {
        if (event.key === "Enter" || event.key === " " || event.key === "Space") go(event);
      }}
    />
  );
}
