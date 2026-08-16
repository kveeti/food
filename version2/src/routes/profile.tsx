import { createAsync } from "@solidjs/router";
import { Suspense } from "solid-js";
import { getCurrentUser } from "../lib/user";

export const route = {
  preload() {
    getCurrentUser();
  },
};

export default function Profile() {
  const user = createAsync(() => getCurrentUser());

  return (
    <main class="mx-auto w-full max-w-4xl px-2 py-10 sm:px-6 pwa:pr-[calc(1rem+var(--safe-area-right))] pwa:pl-[calc(1rem+var(--safe-area-left))] sm:pwa:px-6">
      <h1 class="text-2xl font-semibold tracking-tight">Profile</h1>
      <dl class="mt-8">
        <dt class="text-sm font-medium text-zinc-500">Email</dt>
        <Suspense fallback={<dd class="mt-1 h-6 w-48 animate-pulse bg-zinc-200" />}>
          <dd class="mt-1">{user()?.email}</dd>
        </Suspense>
      </dl>
    </main>
  );
}
