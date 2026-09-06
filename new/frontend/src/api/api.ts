export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: "include",
    ...init,
  });

  if (response.status === 401) {
    globalThis.location.assign("/sign-in");
    throw new ApiError(response.status, "Sign in required");
  }

  const text = await response.text();
  if (!response.ok) {
    throw new ApiError(response.status, text || response.statusText);
  }

  return (text ? JSON.parse(text) : undefined) as T;
}

export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}
