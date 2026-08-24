const ALLOWED_VALUES = new Set(["inherit"]);

export default {
	meta: {
		type: "suggestion",
		docs: {
			description:
				"Disallow hardcoded font families in Svelte <style> blocks; use --font-* variables from theme.css.",
		},
		schema: [],
		messages: {
			font: "Hardcoded font family '{{value}}' is not allowed; use var(--font-body), var(--font-heading) or var(--font-mono).",
		},
	},
	create(context) {
		const sourceCode = context.sourceCode;
		if (!sourceCode.parserServices?.isSvelte) return {};

		return {
			"Program:exit"() {
				const styleContext = sourceCode.parserServices.getStyleContext();
				if (styleContext.status !== "success") return;

				for (const decl of styleContext.sourceAst.nodes ?? []) {
					walkDecls(decl, (node) => {
						if (node.property !== "font-family") return;
						const value = (node.value ?? "").trim();
						if (value.startsWith("var(--font-") || ALLOWED_VALUES.has(value)) {
							return;
						}
						context.report({
							loc: styleNodeLoc(node),
							messageId: "font",
							data: { value },
						});
					});
				}
			},
		};
	},
};

function walkDecls(node, visit) {
	if (node.type === "decl") {
		visit(node);
		return;
	}
	for (const child of node.nodes ?? []) {
		walkDecls(child, visit);
	}
}

function styleNodeLoc(node) {
	if (node.source?.start === undefined) return undefined;
	return {
		start: {
			line: node.source.start.line,
			column: node.source.start.column - 1,
		},
		end: node.source.end
			? {
					line: node.source.end.line,
					column: node.source.end.column,
				}
			: undefined,
	};
}
