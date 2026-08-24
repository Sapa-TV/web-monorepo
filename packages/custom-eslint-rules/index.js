import noColorLiterals from "./rules/no-color-literals.js";
import noExceptionsRule from "./rules/no-exceptions.js";
import noFontLiterals from "./rules/no-font-literals.js";
import propsInlineTypeRule from "./rules/props-inline-type.js";

export const svelteRulesPlugin = {
	meta: { name: "eslint-plugin-sapa", version: "0.1.0" },
	rules: {
		"no-color-literals": noColorLiterals,
		"no-font-literals": noFontLiterals,
		"no-exceptions": noExceptionsRule,
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

export const fontLiterals = {
	plugins: { sapa: svelteRulesPlugin },
	rules: {
		"sapa/no-font-literals": "error",
	},
};

export const propsInlineType = {
	plugins: { sapa: svelteRulesPlugin },
	rules: {
		"sapa/props-inline-type": "error",
	},
};

export const noExceptions = {
	plugins: { sapa: svelteRulesPlugin },
	rules: {
		"sapa/no-exceptions": "error",
	},
};
