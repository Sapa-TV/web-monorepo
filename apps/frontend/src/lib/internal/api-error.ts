import {
	HttpError,
	NetworkError,
	ParseError,
	TimeoutError,
} from "@sapa-tv-ru/api-client";

/**
 * Discriminant for every failure that can cross the API boundary.
 * HTTP-derived kinds are normalized from status codes; transport-level
 * failures get their own names instead of invented pseudo-statuses.
 */
export const ApiErrorKind = {
	BadRequest: "bad_request",
	Unauthorized: "unauthorized",
	Forbidden: "forbidden",
	NotFound: "not_found",
	Conflict: "conflict",
	RateLimited: "rate_limited",
	Server: "server",
	HttpOther: "http_other",
	Timeout: "timeout",
	Network: "network",
	Parse: "parse",
	Unknown: "unknown",
} as const;

export type ApiErrorKind = (typeof ApiErrorKind)[keyof typeof ApiErrorKind];

/** Raw upstream status; present only for the HTTP family of kinds. */
export class ApiError extends Error {
	readonly kind: ApiErrorKind;
	readonly status?: number;
	readonly body?: unknown;

	constructor(kind: ApiErrorKind, status?: number, body?: unknown) {
		super(`api error: ${kind}${status !== undefined ? ` (${status})` : ""}`);
		this.name = "ApiError";
		this.kind = kind;
		this.status = status;
		this.body = body;
	}
}

const HTTP_BAD_REQUEST = 400;
const HTTP_UNAUTHORIZED = 401;
const HTTP_FORBIDDEN = 403;
const HTTP_NOT_FOUND = 404;
const HTTP_CONFLICT = 409;
const HTTP_RATE_LIMITED = 429;
const HTTP_SERVER_MIN = 500; // inclusive
const HTTP_SERVER_MAX_EXCLUSIVE = 600;

function kindFromStatus(status: number): ApiErrorKind {
	switch (status) {
		case HTTP_BAD_REQUEST:
			return ApiErrorKind.BadRequest;
		case HTTP_UNAUTHORIZED:
			return ApiErrorKind.Unauthorized;
		case HTTP_FORBIDDEN:
			return ApiErrorKind.Forbidden;
		case HTTP_NOT_FOUND:
			return ApiErrorKind.NotFound;
		case HTTP_CONFLICT:
			return ApiErrorKind.Conflict;
		case HTTP_RATE_LIMITED:
			return ApiErrorKind.RateLimited;
		default:
			break;
	}
	if (status >= HTTP_SERVER_MIN && status < HTTP_SERVER_MAX_EXCLUSIVE) {
		return ApiErrorKind.Server;
	}
	return ApiErrorKind.HttpOther;
}

/**
 * Normalizes anything the generated client (or an unexpected runtime path)
 * can surface into a stable ApiError. Unknown inputs degrade to the
 * `Unknown` kind instead of being rethrown.
 */
export function normalizeApiError(err: unknown): ApiError {
	if (err instanceof HttpError) {
		return new ApiError(kindFromStatus(err.status), err.status, err.data);
	}
	if (err instanceof TimeoutError) {
		return new ApiError(ApiErrorKind.Timeout, undefined, {
			timeoutMs: err.timeoutMs,
		});
	}
	if (err instanceof NetworkError) {
		return new ApiError(ApiErrorKind.Network, undefined, { cause: err.cause });
	}
	if (err instanceof ParseError) {
		return new ApiError(ApiErrorKind.Parse, undefined, { cause: err.cause });
	}
	return new ApiError(ApiErrorKind.Unknown, undefined, { cause: err });
}
