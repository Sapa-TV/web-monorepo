import { ResultAsync, errAsync, okAsync } from "neverthrow";

export interface ApiConfig {
    baseUrl: string;
    timeoutMs?: number;
}

export class TimeoutError extends Error {
    readonly name = "TimeoutError";
    constructor(readonly timeoutMs: number) {
        super(`Request timed out after ${timeoutMs}ms`);
    }
}

export class NetworkError extends Error {
    readonly name = "NetworkError";
    constructor(readonly cause: unknown) {
        super("Network request failed");
        this.cause = cause;
    }
}

export class ParseError extends Error {
    readonly name = "ParseError";
    constructor(readonly cause: unknown) {
        super("Failed to parse response JSON");
        this.cause = cause;
    }
}

export class HttpError<T = unknown> extends Error {
    readonly name = "HttpError";
    constructor(
        readonly status: number,
        readonly statusText: string,
        readonly data: T
    ) {
        super(`HTTP ${status} ${statusText}`);
    }
}

interface RequestParams {
    path: string;
    method: string;
    body?: unknown;
    query?: Record<string, string | number | boolean | undefined>,
    headers?: Record<string, string>;
}

export class HttpClient<SecurityDataType = unknown> {
    public baseUrl: string;
    private timeoutMs: number;

    constructor(config: ApiConfig) {
        this.baseUrl = config.baseUrl;
        this.timeoutMs = config.timeoutMs ?? 10000;
    }

    // TODO: Add retry logic for transient failures (5xx, network errors)
    // TODO: Add rate limit handling (429 with Retry-After header)
    protected async request<T, E>(params: RequestParams): Promise<ResultAsync<T, E | HttpError<E> | TimeoutError | NetworkError | ParseError>> {
        const { path, method, body, query, headers } = params;

        const url = new URL(path.replace(/^\//, ""), this.baseUrl);

        if (query) {
            Object.entries(query).forEach(([key, value]) => url.searchParams.append(key, String(value)));
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        const headersInit: Record<string, string> = { ...headers };
        if (!(body instanceof FormData) && body) {
            headersInit["Content-Type"] = "application/json";
        }

        const fetchOptions: RequestInit = {
            method,
            headers: headersInit,
            signal: controller.signal,
            body: body instanceof FormData ? body : body ? JSON.stringify(body) : undefined
        };

        let response: Response;
        try {
            response = await fetch(url.toString(), fetchOptions);

            const isNoContent = response.status === 204;
            let data: any = null;

            if (!isNoContent) {
                try {
                    data = await response.json();
                } catch (err) {
                    return errAsync(new ParseError(err));
                }
            }

            if (!response.ok) {
                return errAsync(new HttpError(response.status, response.statusText, data as E));
            }

            return okAsync(data as T);

        } catch (err) {
            if (err instanceof DOMException && err.name === "AbortError") {
                return errAsync(new TimeoutError(this.timeoutMs));
            }
            return errAsync(new NetworkError(err));
        } finally {
            clearTimeout(timeoutId);
        }
    }
}