import { lazy, useEffect } from "react";

import {
  loadApp,
  loadHomePage,
  loadLogPage,
  loadSettingsPage,
  loadSignInPage,
} from "./page-loaders.ts";

export const App = lazy(loadApp);
export const SignInPage = lazy(loadSignInPage);
export const HomePage = lazy(loadHomePage);
export const LogPage = lazy(loadLogPage);
export const SettingsPage = lazy(loadSettingsPage);

export function PreloadPages() {
  useEffect(() => {
    const preload = () => {
      const connection = (
        navigator as Navigator & {
          connection?: { saveData?: boolean; effectiveType?: string };
        }
      ).connection;

      if (
        connection?.saveData ||
        connection?.effectiveType === "slow-2g" ||
        connection?.effectiveType === "2g" ||
        connection?.effectiveType === "3g"
      ) {
        return;
      }

      void Promise.allSettled([
        loadApp(),
        loadSignInPage(),
        loadHomePage(),
        loadLogPage(),
        loadSettingsPage(),
      ]);
    };

    if (typeof window.requestIdleCallback === "function") {
      const id = window.requestIdleCallback(preload);
      return () => window.cancelIdleCallback(id);
    }

    const id = window.setTimeout(preload, 200);
    return () => window.clearTimeout(id);
  }, []);

  return null;
}
