import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const rules = JSON.parse(
	readFileSync(join(root, "arch", "layers.json"), "utf8"),
);

const metadata = JSON.parse(
	execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
		cwd: root,
		encoding: "utf8",
		stdio: ["ignore", "pipe", "inherit"],
		maxBuffer: 64 * 1024 * 1024,
	}),
);

const workspaceIds = new Set(metadata.workspace_members);
const packages = metadata.packages.filter((pkg) => workspaceIds.has(pkg.id));
const workspaceNames = new Set(packages.map((pkg) => pkg.name));

const problems = [];

for (const name of Object.keys(rules.crates)) {
	if (!workspaceNames.has(name)) {
		problems.push(
			`arch/layers.json: crate '${name}' is not a workspace member`,
		);
	}
}
for (const name of workspaceNames) {
	if (!Object.hasOwn(rules.crates, name)) {
		problems.push(`arch/layers.json: workspace member '${name}' has no rule`);
	}
}
for (const name of rules.optional_only ?? []) {
	if (!workspaceNames.has(name)) {
		problems.push(
			`arch/layers.json: optional_only names unknown crate '${name}'`,
		);
	}
}

const owned = rules.owned_external ?? {};
const optionalOnly = new Set(rules.optional_only ?? []);
let internalEdges = 0;

for (const pkg of packages) {
	const rule = rules.crates[pkg.name];
	if (!rule) continue;

	const allowed = new Set(rule.depends_on ?? []);
	const allowedDev = new Set(rule.dev_depends_on ?? []);
	const declared = new Set();

	for (const dep of pkg.dependencies) {
		const isInternal = workspaceNames.has(dep.name);
		declared.add(dep.name);

		if (isInternal) {
			if (optionalOnly.has(dep.name) && !dep.optional && dep.kind !== "dev") {
				problems.push(
					`${pkg.name} -> ${dep.name}: only an optional dependency is allowed here`,
				);
			}
			if (dep.kind === "dev") {
				if (!allowedDev.has(dep.name)) {
					problems.push(
						`${pkg.name} (dev) -> ${dep.name}: edge is not allowed`,
					);
				}
			} else if (!allowed.has(dep.name)) {
				const kind = dep.kind === "build" ? " (build)" : "";
				problems.push(`${pkg.name}${kind} -> ${dep.name}: edge is not allowed`);
			}
			internalEdges += 1;
		} else if (
			Object.hasOwn(owned, dep.name) &&
			!owned[dep.name].includes(pkg.name)
		) {
			problems.push(
				`${pkg.name} -> ${dep.name}: owned by ${owned[dep.name].join(", ") || "no crate"}`,
			);
		}
	}

	for (const name of allowed) {
		if (!declared.has(name) && workspaceNames.has(name)) {
			problems.push(
				`arch/layers.json: ${pkg.name} -> ${name} is allowed but not declared`,
			);
		}
	}
	for (const name of allowedDev) {
		if (!declared.has(name) && workspaceNames.has(name)) {
			problems.push(
				`arch/layers.json: ${pkg.name} (dev) -> ${name} is allowed but not declared`,
			);
		}
	}
}

if (problems.length > 0) {
	for (const problem of problems.sort()) {
		console.error(`layers: ${problem}`);
	}
	console.error(`layers: ${problems.length} violation(s)`);
	process.exit(1);
}

console.log(
	`layers: ${packages.length} crates, ${internalEdges} internal edges, ${Object.keys(owned).length} owned externals — ok`,
);
