import noColorLiterals from "./rules/no-color-literals.js";
import propsInlineTypeRule from "./rules/props-inline-type.js";

export const svelteRulesPlugin = {
	meta: { name: "eslint-plugin-sapa", version: "0.1.0" },
	rules: {
		"no-color-literals": noColorLiterals,
		"props-inline-type": propsInlineTypeRule,
	},
};

export const colorLiterals = {
	plugins: { sapa: svelteRulesPlugin },
	rules: {
		"sapa/no-color-literals": "error",
		"svelte/no-inline-styles": "error",
	},
};

export const propsInlineType = {
	plugins: { sapa: svelteRulesPlugin },
	rules: {
		"sapa/props-inline-type": "error",
	},
};
