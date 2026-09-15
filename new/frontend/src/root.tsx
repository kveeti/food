import { StrictMode, Suspense } from "react";
import { Route, Router, Switch } from "wouter";

import { App, PreloadPages, SignInPage } from "./features/pages.tsx";

export function Root(props: { ssrPath?: string }) {
  return (
    <StrictMode>
      <Router ssrPath={props.ssrPath}>
        <Suspense fallback={null}>
          <Switch>
            <Route path="/sign-in">
              <SignInPage />
              <PreloadPages />
            </Route>
            <Route>
              <App />
            </Route>
          </Switch>
        </Suspense>
      </Router>
    </StrictMode>
  );
}
