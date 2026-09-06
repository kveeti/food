import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Redirect, Route, Router, Switch } from "wouter";

import SignInPage from "./features/auth/sign-in-page.tsx";
import HomePage from "./features/home/home-page.tsx";

import "./styles.css";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Router>
        <Switch>
          <Route path="/sign-in" component={SignInPage} />
          <Route path="/" component={HomePage} />
          <Route>
            <Redirect to="/" />
          </Route>
        </Switch>
      </Router>
    </QueryClientProvider>
  </StrictMode>,
);
