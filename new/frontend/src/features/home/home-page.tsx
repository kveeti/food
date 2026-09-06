import { useQuery } from "@tanstack/react-query";

import { api } from "../../api/api.ts";

type User = {
  id: string;
  email: string | null;
};

export default function HomePage() {
  const user = useQuery({
    queryKey: ["me"],
    queryFn: () => api<User>("/api/me"),
    retry: false,
  });

  if (user.isPending) return null;
  if (user.isError) {
    return (
      <main className="mx-auto max-w-[var(--page-width)] px-7 py-8">
        Could not load Food.
      </main>
    );
  }

  return (
    <main className="mx-auto max-w-[var(--page-width)] px-7 py-8">
      <h1 className="text-xl font-semibold text-gray-950">Hello world</h1>
    </main>
  );
}
