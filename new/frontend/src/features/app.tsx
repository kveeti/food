import { Redirect, Route, Switch, useLocation } from "wouter";

import { useMeQuery } from "../api/user.ts";
import { ImmediateNavLink } from "../ui/immediate-nav-link.tsx";
import HomePage from "./home/home-page.tsx";
import { I18n } from "./i18n/i18n.tsx";
import SettingsPage from "./settings/settings-page.tsx";

export default function App() {
  const me = useMeQuery();
  const [location] = useLocation();

  if (me.isPending) return null;
  if (me.isError) {
    return (
      <main className="mx-auto max-w-[var(--page-width)] px-4 py-8 sm:px-7">
        Error loading Food.
      </main>
    );
  }

  const needsSetup = !me.data.locale || !me.data.timezone;
  if (needsSetup && location !== "/settings") {
    return <Redirect to="/settings" />;
  }

  return (
    <div>
      <AppNav setup={needsSetup} />
      <Switch>
        <Route path="/settings">
          <SettingsPage user={me.data} />
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
    </div>
  );
}

function AppNav(props: { setup: boolean }) {
  return (
    <nav className="fixed inset-x-0 bottom-0 z-10 h-[var(--nav-height)] border-t border-gray-200 bg-gray-100/80 pb-[env(safe-area-inset-bottom)] text-gray-950 backdrop-blur-md sm:sticky sm:top-0 sm:border-t-0 sm:pb-0">
      <div className="mx-auto flex h-full max-w-[var(--page-width)] items-stretch px-2 sm:px-4">
        {!props.setup && (
          <>
            <ImmediateNavLink
              href="/"
              className="inline-flex items-center px-3 text-sm text-inherit outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-200/80 focus-visible:outline-gray-500"
            >
              Today
            </ImmediateNavLink>
            <ImmediateNavLink
              href="/settings"
              className="inline-flex items-center px-3 text-sm text-inherit outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-200/80 focus-visible:outline-gray-500"
            >
              Settings
            </ImmediateNavLink>
          </>
        )}
        <form method="post" action="/logout" className="ml-auto h-full">
          <button
            type="submit"
            className="inline-flex h-full items-center px-3 text-sm text-inherit outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-200/80 focus-visible:outline-gray-500"
          >
            Log out
          </button>
        </form>
      </div>
    </nav>
  );
}
