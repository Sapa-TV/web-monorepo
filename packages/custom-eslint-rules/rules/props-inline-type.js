export default {
	meta: {
		type: "suggestion",
		docs: {
			description:
				"Require component props to be declared as an internal `interface Props` with `let {...}: Props = $props()`.",
		},
		schema: [],
		messages: {
			noAnnotation:
				"Props must be destructured with an explicit type annotation: `let { ... }: Props = $props()`.",
			inlineType:
				"Inline props types are forbidden; declare `interface Props { ... }` and use `: Props`.",
			inventedName:
				"Invented props type name '{{name}}' is forbidden; the interface must be called `Props`.",
			typeAlias: "Props type must be `interface Props`, not a `type` alias.",
			exported: "The `Props` interface must not be exported.",
		},
	},
	create(context) {
		const sourceCode = context.sourceCode;
		if (!sourceCode.parserServices?.isSvelte) return {};

		function report(node, messageId, data = {}) {
			context.report({ node, messageId, data });
		}

		function isPropsCall(init) {
			return (
				init?.type === "CallExpression" &&
				init.callee?.type === "Identifier" &&
				init.callee.name === "$props"
			);
		}

		return {
			TSInterfaceDeclaration(node) {
				const name = node.id.name;
				if (name !== "Props" && name.endsWith("Props")) {
					report(node, "inventedName", { name });
				}
				if (name === "Props" && isExported(node)) {
					report(node, "exported");
				}
			},
			TSTypeAliasDeclaration(node) {
				const name = node.id.name;
				if (name === "Props" || name.endsWith("Props")) {
					report(node, "typeAlias");
				}
			},
			VariableDeclarator(node) {
				if (!isPropsCall(node.init)) return;
				const { id } = node;
				if (id.type !== "ObjectPattern") {
					report(node.init, "inlineType");
					return;
				}
				if (node.init.typeParameters) {
					report(node.init, "inlineType");
					return;
				}
				const annotation = id.typeAnnotation?.typeAnnotation;
				if (!annotation) {
					report(id, "noAnnotation");
					return;
				}
				if (
					annotation.type === "TSTypeLiteral" ||
					annotation.type === "TSIntersectionType" ||
					annotation.type === "TSUnionType"
				) {
					report(annotation, "inlineType");
					return;
				}
				if (
					annotation.type === "TSTypeReference" &&
					(annotation.typeName?.type !== "Identifier" ||
						annotation.typeName.name !== "Props")
				) {
					const name =
						annotation.typeName?.type === "Identifier"
							? annotation.typeName.name
							: String(annotation.typeName?.name ?? "");
					report(
						annotation,
						name.endsWith("Props") ? "inventedName" : "noAnnotation",
						{
							name,
						},
					);
				}
			},
		};
	},
};

function isExported(node) {
	return (
		node.export === true ||
		node.parent?.type === "ExportNamedDeclaration" ||
		node.parent?.type === "ExportDefaultDeclaration"
	);
}
