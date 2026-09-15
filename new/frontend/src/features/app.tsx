import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { motion } from "framer-motion";
import { Suspense } from "react";
import { Redirect, Route, Switch, useLocation } from "wouter";

import { useMeQuery } from "../api/user.ts";
import { useIsReducedMotion } from "../lib/use-is-reduced-motion.ts";
import { ImmediateNavLink } from "../ui/immediate-nav-link.tsx";
import { Toaster } from "../ui/toaster/toaster.tsx";
import { I18n } from "./i18n/i18n.tsx";
import { loadHomePage, loadLogPage, loadSettingsPage } from "./page-loaders.ts";
import { HomePage, LogPage, PreloadPages, SettingsPage } from "./pages.tsx";

const navItems = [
  { href: "/", label: "Today", load: loadHomePage },
  { href: "/log", label: "Log", load: loadLogPage },
  { href: "/settings", label: "You", load: loadSettingsPage },
];

const queryClient = new QueryClient();

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppContent />
      <ReactQueryDevtools />
    </QueryClientProvider>
  );
}

function AppContent() {
  const me = useMeQuery();
  const [location] = useLocation();

  if (me.isPending) return null;
  if (me.isError) {
    return (
      <main className="h-full overflow-y-auto overscroll-contain sm:[scrollbar-gutter:stable_both-edges]">
        <div className="mx-auto max-w-[var(--page-max-width)] px-[var(--page-padding)] py-8">
          Error loading Food.
        </div>
      </main>
    );
  }

  const needsSetup = !me.data.locale || !me.data.timezone;
  if (needsSetup && location !== "/settings") {
    return <Redirect to="/settings" />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden [--nav-clearance:calc(3.75rem_+_env(safe-area-inset-bottom,0px))] sm:[--nav-clearance:calc(4.25rem_+_env(safe-area-inset-bottom,0px))]">
      <Toaster />
      <AppNav setup={needsSetup} />
      <Suspense fallback={null}>
        <Switch>
          <Route path="/settings">
            <SettingsPage user={me.data} />
          </Route>
          <Route path="/log">
            <LogPage />
          </Route>
          <Route path="/">
            <I18n locale={me.data.locale!} timeZone={me.data.timezone!}>
              <HomePage />
            </I18n>
          </Route>
          <Route>
            <Redirect to="/" />
          </Route>
        </Switch>
        <PreloadPages />
      </Suspense>
    </div>
  );
}

function AppNav(props: { setup: boolean }) {
  const [location] = useLocation();
  const isReducedMotion = useIsReducedMotion();
  const path = location.split("?")[0];
  const selectedIndex = navItems.findIndex((item) => item.href === path);

  if (props.setup) return null;

  return (
    <nav
      aria-label="Main"
      className="fixed bottom-[env(safe-area-inset-bottom,0px)] left-1/2 z-30 w-[min(18rem,calc(100%_-_1.5rem))] -translate-x-1/2 sm:bottom-[max(0.75rem,env(safe-area-inset-bottom,0px))]"
    >
      <div className="overflow-hidden rounded-full border border-gray-250 bg-gray-100/85 text-gray-800 shadow-float backdrop-blur-xl">
        <div className="relative flex p-1">
          <motion.span
            initial={false}
            aria-hidden="true"
            className="pointer-events-none absolute inset-y-1 left-1 w-[calc((100%_-_0.5rem)/3)] rounded-full bg-gray-250"
            animate={{ x: `${selectedIndex * 100}%` }}
            transition={
              isReducedMotion
                ? { duration: 0 }
                : { type: "spring", stiffness: 500, damping: 38 }
            }
          />
          {navItems.map((item) => {
            const selected = path === item.href;
            return (
              <ImmediateNavLink
                key={item.href}
                href={item.href}
                onPreload={() => {
                  void item.load().catch(() => {});
                }}
                aria-current={selected ? "page" : undefined}
                className="relative flex flex-1 items-center justify-center rounded-full text-sm font-medium outline-2 outline-transparent [-webkit-tap-highlight-color:transparent] focus-visible:outline-gray-500"
              >
                <span className="px-4 py-3.5 sm:py-2">{item.label}</span>
              </ImmediateNavLink>
            );
          })}
        </div>
      </div>
    </nav>
  );
}
