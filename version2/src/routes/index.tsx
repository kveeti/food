import { createAsync, useSubmission } from "@solidjs/router";
import { getWaterMlToday, logWater } from "../lib/water";
import { format } from 'date-fns'
import { Show } from "solid-js";

export default function Home() {
  return (
    <main class="mx-auto w-full max-w-4xl px-2 py-10 sm:px-6 pwa:pr-[calc(1rem+var(--safe-area-right))] pwa:pl-[calc(1rem+var(--safe-area-left))] sm:pwa:px-6">
      <Water />
    </main>
  );
}

function Water() {
  const submission = useSubmission(logWater);
  const waterMlToday = createAsync(() => getWaterMlToday(), {deferStream: true})

  return (
    <div>
      <h2>Water</h2>

        <p class="mt-1">{waterMlToday()} ml</p>

      <form action={logWater} method="post" class="flex flex-col gap-2 max-w-70">
        <div class="flex flex-col gap-2">
          <input
            name="amountMl"
            type="number"
            placeholder="Amount ml"
            autofocus={!!submission.result}
          />
          <input
            name="consumedAt"
            type="datetime-local"
            placeholder="Consumed at"
            value={format(new Date(), "yyyy-MM-dd'T'HH:mm")}
          />
        </div>

        <button>Log</button>
      </form>

      <Show when={submission.error}>
        {submission.error?.message}
      </Show>
    </div>
  );
}
