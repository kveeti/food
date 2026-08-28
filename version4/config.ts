function requireEnv(name: string): string {
  const value = Deno.env.get(name);
  if (!value) {
    throw new Error(`${name} must be set`);
  }
  return value;
}

const port = Number(Deno.env.get("PORT") ?? "8000");
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  throw new Error("PORT must be a number between 1 and 65535");
}

export const config = {
  databaseUrl: requireEnv("DATABASE_URL"),
  port,
} as const;
