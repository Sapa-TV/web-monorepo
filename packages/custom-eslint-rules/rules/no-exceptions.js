export default {
	meta: {
		type: "problem",
		docs: {
			description:
				"Disallow throw, try/catch and Promise .catch(); failures must flow through neverthrow Results. Allowed only in the designated internal boundary folder.",
		},
		schema: [],
		messages: {
			noThrow:
				"Do not use `throw`. Return a neverthrow Result (err/ResultAsync.errAsync) instead.",
			noTryCatch:
				"Do not use try/catch here. Handle failures with Result (mapErr/andThen/safeTry). Only src/lib/internal/ may establish try boundaries.",
			noDotCatch:
				"Do not use `.catch()` on promises. Normalize the failure into a Result at the boundary.",
		},
	},
	create(context) {
		return {
			ThrowStatement(node) {
				context.report({ node, messageId: "noThrow" });
			},
			TryStatement(node) {
				context.report({ node, messageId: "noTryCatch" });
			},
			CallExpression(node) {
				const callee = node.callee;
				if (
					callee.type === "MemberExpression" &&
					callee.property.type === "Identifier" &&
					callee.property.name === "catch"
				) {
					context.report({ node, messageId: "noDotCatch" });
				}
			},
		};
	},
};
