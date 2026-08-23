import { describe, expect, it } from "vitest";
import {
	HttpError,
	NetworkError,
	ParseError,
	TimeoutError,
} from "@sapa-tv-ru/api-client";

import { ApiError, ApiErrorKind, normalizeApiError } from "./api-error";

describe("normalizeApiError", () => {
	it("maps exact statuses to their kinds and keeps the raw status", () => {
		const cases = [
			[400, ApiErrorKind.BadRequest],
			[401, ApiErrorKind.Unauthorized],
			[403, ApiErrorKind.Forbidden],
			[404, ApiErrorKind.NotFound],
			[409, ApiErrorKind.Conflict],
			[429, ApiErrorKind.RateLimited],
		] as const;

		for (const [status, kind] of cases) {
			const normalized = normalizeApiError(
				new HttpError(status, "text", { detail: "x" }),
			);
			expect(normalized.kind).toBe(kind);
			expect(normalized.status).toBe(status);
		}
	});

	it("collapses 5xx into Server", () => {
		for (const status of [500, 502, 503, 599]) {
			const normalized = normalizeApiError(new HttpError(status, "text", null));
			expect(normalized.kind).toBe(ApiErrorKind.Server);
			expect(normalized.status).toBe(status);
		}
	});

	it("maps unknown statuses to HttpOther while preserving them", () => {
		for (const status of [302, 418, 451]) {
			const normalized = normalizeApiError(
				new HttpError(status, "text", "payload"),
			);
			expect(normalized.kind).toBe(ApiErrorKind.HttpOther);
			expect(normalized.status).toBe(status);
		}
	});

	it("keeps the response body accessible", () => {
		const body = { error: "detail" };
		const normalized = normalizeApiError(new HttpError(400, "bad", body));
		expect(normalized.body).toEqual(body);
	});

	it("normalizes transport failures without inventing statuses", () => {
		const timeout = normalizeApiError(new TimeoutError(10_000));
		expect(timeout.kind).toBe(ApiErrorKind.Timeout);
		expect(timeout.status).toBeUndefined();

		const network = normalizeApiError(new NetworkError(new Error("offline")));
		expect(network.kind).toBe(ApiErrorKind.Network);

		const parse = normalizeApiError(new ParseError(new Error("bad json")));
		expect(parse.kind).toBe(ApiErrorKind.Parse);
	});

	it("degrades unknown inputs to the Unknown kind", () => {
		const normalized = normalizeApiError(new Error("something local"));
		expect(normalized.kind).toBe(ApiErrorKind.Unknown);
		expect(normalized.status).toBeUndefined();
	});

	it("produces an ApiError instance whose message names the kind", () => {
		const normalized = normalizeApiError(new HttpError(401, "unauth", null));
		expect(normalized).toBeInstanceOf(ApiError);
		expect(normalized.message).toContain("unauthorized");
	});
});
