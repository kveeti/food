import {
  Navigate,
  Router,
  type RouteDefinition,
} from '@solidjs/router';
import { Show, type Component, type JSX } from 'solid-js';

import { useSettings } from './app.data';
import { ErrorBlock, Loading } from './components/ui';
import { Nav, Page, Shell } from './layout';
import { SettingsPage } from './pages/settings-page';
import { TrackerPage } from './pages/tracker-page';

export const appRoutes: RouteDefinition[] = [
  {
    path: '/',
    component: TrackerPage,
  },
  {
    path: '/settings',
    component: SettingsPage,
  },
  {
    path: '**',
    component: () => <Navigate href="/" />,
  },
];

export const setupRoutes: RouteDefinition[] = [
  {
    path: '/settings',
    component: SettingsPage,
  },
  {
    path: '**',
    component: () => <Navigate href="/settings" />,
  },
];

const App: Component = () => {
  const settings = useSettings();

  return (
    <Show
      when={!settings.isLoading}
      fallback={
        <Shell>
          <Loading label="Loading" />
        </Shell>
      }
    >
      <Show
        when={!settings.isError}
        fallback={
          <Shell>
            <ErrorBlock error={settings.error} />
          </Shell>
        }
      >
        <Show when={settings.data !== null} fallback={<SetupRouter />}>
          <AppRouter />
        </Show>
      </Show>
    </Show>
  );
};

const SetupRouter: Component = () => (
  <Router root={SetupShell}>
    {setupRoutes}
  </Router>
);

const AppRouter: Component = () => (
  <Router root={AppShell}>
    {appRoutes}
  </Router>
);

const SetupShell: Component<{ children?: JSX.Element }> = (props) => (
  <Shell>
    <Page>{props.children}</Page>
  </Shell>
);

const AppShell: Component<{ children?: JSX.Element }> = (props) => (
  <Shell>
    <Nav />
    <Page>{props.children}</Page>
  </Shell>
);

export default App;
