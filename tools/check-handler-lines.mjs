import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";

const THRESHOLD = 20;

const require = createRequire(import.meta.url);
const { resolveBinaryPath } = require("@ast-grep/cli/postinstall");
const astGrep = resolveBinaryPath();
if (!astGrep) {
	console.error(
		"ast-grep native binary not found; reinstall dependencies without --no-optional",
	);
	process.exit(1);
}

let json;
try {
	json = execFileSync(
		astGrep,
		[
			"scan",
			"--json",
			"-r",
			".sg/selectors/max-handler-lines.yml",
			"crates/api/src/api",
			"crates/api/src/widget_api",
		],
		{ encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
	);
} catch (e) {
	if (!e.stdout?.trim()) {
		console.error(e.stderr?.trim() || e.message);
		process.exit(1);
	}
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
