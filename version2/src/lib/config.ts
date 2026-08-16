function requiredEnv(name: string) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

console.log("validating config...");

const appUrl = requiredEnv("APP_URL");
const parsedAppUrl = new URL(appUrl);

const config = {
  appUrl,
  databaseUrl: requiredEnv("DATABASE_URL"),
  migrationsPath: requiredEnv("MIGRATIONS_PATH"),
  oidcIssuer: requiredEnv("OIDC_ISSUER"),
  oidcClientId: requiredEnv("OIDC_CLIENT_ID"),
  oidcClientSecret: requiredEnv("OIDC_CLIENT_SECRET"),
  oidcRedirectUrl: requiredEnv("OIDC_REDIRECT_URL"),
  sessionSecret: requiredEnv("SESSION_SECRET"),
  secureCookies: parsedAppUrl.protocol === "https:",
};

if (config.sessionSecret.length < 32) {
  throw new Error("SESSION_SECRET must contain at least 32 characters");
}

console.info("config validated:", {
  ...config,
  databaseUrl: "[redacted]",
  oidcClientSecret: "[redacted]",
  sessionSecret: "[redacted]",
});

export { config };
