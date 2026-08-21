import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const THRESHOLD = 20;

const require = createRequire(import.meta.url);
const cliPkg = dirname(require.resolve("@ast-grep/cli/package.json"));
const astGrep = join(cliPkg, "ast-grep");

let json;
try {
	json = execFileSync(
		process.execPath,
		[
			astGrep,
			"scan",
			"--json",
			"-c",
			"sgconfig.yml",
			"apps/backend/src/api",
			"apps/backend/src/widget_api",
		],
		{ encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
	);
} catch (e) {
	json = e.stdout;
}

const findings = JSON.parse(json)
	.map((r) => {
		const textLines = r.text.split("\n");
		const bodyStart = textLines.findIndex((l) => l.trimEnd().endsWith("{"));
		return { ...r, lines: textLines.length - bodyStart };
	})
	.filter((r) => r.lines > THRESHOLD);

for (const f of findings) {
	const file = f.fileRelative ?? f.file;
	console.error(
		`${file}:${f.range.start.line + 1}: handler is ${f.lines} lines (max ${THRESHOLD})`,
	);
}

if (findings.length > 0) {
	console.error(
		`max-handler-lines: ${findings.length} handler(s) exceed ${THRESHOLD} lines`,
	);
	process.exit(1);
}
console.log(`max-handler-lines: OK (all handler bodies <= ${THRESHOLD} lines)`);
