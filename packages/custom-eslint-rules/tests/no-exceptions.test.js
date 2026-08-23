import { RuleTester } from "eslint";
import { test } from "node:test";

import rule from "../rules/no-exceptions.js";

const ruleTester = new RuleTester();

test("no-exceptions", () => {
	ruleTester.run("no-exceptions", rule, {
		valid: [
			"const value = result.unwrapOr(0);",
			"const res = await api.ping(); if (res.isErr()) showError(res.error);",
			"promise.then(onOk, onFail);",
			"result.mapErr(handleFailure);",
			"queue.on('failed', handler);",
		],
		invalid: [
			{
				code: "throw new Error('boom');",
				errors: [{ messageId: "noThrow" }],
			},
			{
				code: "try { run(); } catch (e) { log(e); }",
				errors: [{ messageId: "noTryCatch" }],
			},
			{
				code: "try { run(); } finally { cleanup(); }",
				errors: [{ messageId: "noTryCatch" }],
			},
			{
				code: "async function f() { try { await api.x(); } catch (e) { return null; } }",
				errors: [{ messageId: "noTryCatch" }],
			},
			{
				code: "load().catch(setError);",
				errors: [{ messageId: "noDotCatch" }],
			},
			{
				code: "promise?.catch(handler);",
				errors: [{ messageId: "noDotCatch" }],
			},
			{
				code: "const handle = () => { throw failures.missing(); };",
				errors: [{ messageId: "noThrow" }],
			},
		],
	});
});
