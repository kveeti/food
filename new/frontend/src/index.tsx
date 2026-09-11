import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Route, Router, Switch } from "wouter";

import App from "./features/app.tsx";
import SignInPage from "./features/auth/sign-in-page.tsx";

import "./styles.css";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Router>
        <Switch>
          <Route path="/sign-in" component={SignInPage} />
          <Route>
            <App />
          </Route>
        </Switch>
      </Router>
      <ReactQueryDevtools />
    </QueryClientProvider>
  </StrictMode>,
);
