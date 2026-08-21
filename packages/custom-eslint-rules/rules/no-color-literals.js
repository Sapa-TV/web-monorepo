const HEX_RE = /(?<![\w])#[0-9a-fA-F]{3,8}\b/g;
const FUNC_RE = /\b(?:rgb|rgba|hsl|hsla|oklch|oklab|lab|lch|hwb|color)\(/g;

const NAMED_COLORS = [
	"aliceblue",
	"antiquewhite",
	"aqua",
	"aquamarine",
	"azure",
	"beige",
	"bisque",
	"black",
	"blanchedalmond",
	"blue",
	"blueviolet",
	"brown",
	"burlywood",
	"cadetblue",
	"chartreuse",
	"chocolate",
	"coral",
	"cornflowerblue",
	"cornsilk",
	"crimson",
	"cyan",
	"darkblue",
	"darkcyan",
	"darkgoldenrod",
	"darkgray",
	"darkgreen",
	"darkgrey",
	"darkkhaki",
	"darkmagenta",
	"darkolivegreen",
	"darkorange",
	"darkorchid",
	"darkred",
	"darksalmon",
	"darkseagreen",
	"darkslateblue",
	"darkslategray",
	"darkslategrey",
	"darkturquoise",
	"darkviolet",
	"deeppink",
	"deepskyblue",
	"dimgray",
	"dimgrey",
	"dodgerblue",
	"firebrick",
	"floralwhite",
	"forestgreen",
	"fuchsia",
	"gainsboro",
	"ghostwhite",
	"gold",
	"goldenrod",
	"gray",
	"green",
	"greenyellow",
	"grey",
	"honeydew",
	"hotpink",
	"indianred",
	"indigo",
	"ivory",
	"khaki",
	"lavender",
	"lavenderblush",
	"lawngreen",
	"lemonchiffon",
	"lightblue",
	"lightcoral",
	"lightcyan",
	"lightgoldenrodyellow",
	"lightgray",
	"lightgreen",
	"lightgrey",
	"lightpink",
	"lightsalmon",
	"lightseagreen",
	"lightskyblue",
	"lightslategray",
	"lightslategrey",
	"lightsteelblue",
	"lightyellow",
	"lime",
	"limegreen",
	"linen",
	"magenta",
	"maroon",
	"mediumaquamarine",
	"mediumblue",
	"mediumorchid",
	"mediumpurple",
	"mediumseagreen",
	"mediumslateblue",
	"mediumspringgreen",
	"mediumturquoise",
	"mediumvioletred",
	"midnightblue",
	"mintcream",
	"mistyrose",
	"moccasin",
	"navajowhite",
	"navy",
	"oldlace",
	"olive",
	"olivedrab",
	"orange",
	"orangered",
	"orchid",
	"palegoldenrod",
	"palegreen",
	"paleturquoise",
	"palevioletred",
	"papayawhip",
	"peachpuff",
	"peru",
	"pink",
	"plum",
	"powderblue",
	"purple",
	"rebeccapurple",
	"red",
	"rosybrown",
	"royalblue",
	"saddlebrown",
	"salmon",
	"sandybrown",
	"seagreen",
	"seashell",
	"sienna",
	"silver",
	"skyblue",
	"slateblue",
	"slategray",
	"slategrey",
	"snow",
	"springgreen",
	"steelblue",
	"tan",
	"teal",
	"thistle",
	"tomato",
	"turquoise",
	"violet",
	"wheat",
	"white",
	"whitesmoke",
	"yellow",
	"yellowgreen",
];

const NAMED_RE = new RegExp(`\\b(?:${NAMED_COLORS.join("|")})\\b`, "g");

function findAll(value) {
	const matches = [];
	for (const re of [HEX_RE, FUNC_RE, NAMED_RE]) {
		re.lastIndex = 0;
		let m;
		while ((m = re.exec(value)) !== null) {
			const literal = m[0].replace(/\($/, "");
			matches.push({ index: m.index, literal });
		}
	}
	matches.sort((a, b) => a.index - b.index);
	const deduped = [];
	for (const match of matches) {
		if (
			deduped.length === 0 ||
			deduped[deduped.length - 1].index !== match.index
		) {
			deduped.push(match);
		}
	}
	return deduped;
}

export default {
	meta: {
		type: "suggestion",
		docs: {
			description:
				"Disallow color literals in Svelte <style> blocks; use CSS variables instead.",
		},
		schema: [],
		messages: {
			color:
				"Color literal '{{literal}}' is not allowed; use a CSS variable from theme.css.",
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
						for (const match of findAll(node.value ?? "")) {
							context.report({
								loc: styleNodeLoc(node),
								messageId: "color",
								data: { literal: match.literal },
							});
						}
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
