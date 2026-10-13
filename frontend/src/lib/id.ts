const devPrefix = Math.random().toString(36).slice(2);
let nextDevId = 0;

export function createId() {
  if (import.meta.env.DEV) {
    nextDevId += 1;
    return `dev-${devPrefix}-${nextDevId}`;
  }

  return crypto.randomUUID();
}
