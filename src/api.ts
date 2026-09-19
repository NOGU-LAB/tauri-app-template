const backendTokenHeader = "X-Backend-Token";

type ErrorResponse = {
  error?: string;
};

export async function requestJSON<T>(
  apiBase: string,
  token: string,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set(backendTokenHeader, token);
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const response = await fetch(`${apiBase}${path}`, { ...init, headers });
  if (!response.ok) {
    let message = `APIエラー (${response.status})`;
    try {
      const body = (await response.json()) as ErrorResponse;
      if (body.error) message = body.error;
    } catch {
      // JSONでないエラーレスポンスではステータス由来のメッセージを使う。
    }
    throw new Error(message);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}
