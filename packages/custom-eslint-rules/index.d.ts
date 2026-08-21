import type { Linter } from "eslint";

type SapaPlugin = NonNullable<Linter.Config["plugins"]>[string];

export declare const svelteRulesPlugin: SapaPlugin;
export declare const colorLiterals: Linter.Config;
export declare const propsInlineType: Linter.Config;
