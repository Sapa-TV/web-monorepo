/** @type {import('stylelint').Config} */
export default {
	ignoreFiles: ["src/styles/theme.css"],
	rules: {
		"declaration-property-value-allowed-list": {
			"/^(color|background|background-color|background-image|border-color|outline-color|fill|stroke|text-decoration-color)$/":
				["/\\bvar\\(--/", "transparent", "currentColor", "inherit", "none"],
			"/^border(-(top|right|bottom|left))$/": [
				"/\\bvar\\(--/",
				"none",
				"/^0$/",
			],
			"font-family": ["/^var\\(--font-/"],
		},
	},
};
