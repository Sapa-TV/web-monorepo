import { describe, expect, it } from "vitest";
import { decodeApplicationKey } from "./push-browser";

describe("decodeApplicationKey", () => {
	it("decodes a base64url key without padding", () => {
		expect(Array.from(decodeApplicationKey("AQID") ?? [])).toEqual([1, 2, 3]);
	});

	it("returns null for malformed input", () => {
		expect(decodeApplicationKey("%%%")).toBeNull();
	});
});
