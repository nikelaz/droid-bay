export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}

export async function request<T>(
  path: string,
  options?: RequestInit,
): Promise<T> {
  const response = await fetch(path, {
    ...options,
    headers: { "Content-Type": "application/json", ...options?.headers },
  });
  const body = await response.json().catch(() => null);
  if (!response.ok)
    throw new ApiError(
      body?.error || `Request failed (${response.status})`,
      response.status,
    );
  if (body === null)
    throw new Error("The server returned an unexpected response.");
  return body as T;
}

export const errorMessage = (error: unknown) =>
  error instanceof Error
    ? error.message
    : "Something went wrong. Please try again.";
