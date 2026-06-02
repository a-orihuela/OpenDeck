#!/usr/bin/env -S deno run -A

/**
 * Fails when changed Svelte files contain hardcoded UI literals that should be translated.
 * Scope is intentionally limited to files changed vs HEAD to avoid blocking on legacy debt.
 */

type Violation = {
	file: string;
	line: number;
	reason: string;
	content: string;
};

async function changedFilesSinceHead(): Promise<string[]> {
	const cmd = new Deno.Command("git", {
		args: ["diff", "--name-only", "HEAD"],
		stdout: "piped",
		stderr: "null",
	});
	const { code, stdout } = await cmd.output();
	if (code !== 0) return [];
	const text = new TextDecoder().decode(stdout).trim();
	if (!text) return [];
	return text.split("\n").map((line) => line.trim()).filter(Boolean);
}

function shouldCheck(file: string): boolean {
	if (!file.endsWith(".svelte")) return false;
	return file.startsWith("src/components/") || file.startsWith("src/routes/");
}

function scanFile(file: string, source: string): Violation[] {
	const violations: Violation[] = [];
	const lines = source.split("\n");

	const attrLiteral = /(placeholder|title|aria-label)\s*=\s*"([^"{][^"]*[A-Za-zÁÉÍÓÚÑáéíóúñ][^"]*)"/;
	const textNodeLiteral = />\s*([A-Za-zÁÉÍÓÚÑáéíóúñ][^<{]*)\s*</;

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		if (line.includes("i18n-ignore")) continue;

		if (attrLiteral.test(line)) {
			violations.push({
				file,
				line: i + 1,
				reason: "Hardcoded attribute text",
				content: line.trim(),
			});
		}

		if (textNodeLiteral.test(line)) {
			violations.push({
				file,
				line: i + 1,
				reason: "Hardcoded text node",
				content: line.trim(),
			});
		}
	}

	return violations;
}

const changed = await changedFilesSinceHead();
const targets = changed.filter(shouldCheck);

if (targets.length === 0) {
	console.log("i18n-check: no changed Svelte files to scan");
	Deno.exit(0);
}

const violations: Violation[] = [];
for (const file of targets) {
	const source = await Deno.readTextFile(file);
	violations.push(...scanFile(file, source));
}

if (violations.length > 0) {
	console.error("i18n-check: found untranslated literals in changed files");
	for (const v of violations) {
		console.error(`- ${v.file}:${v.line} ${v.reason}`);
		console.error(`  ${v.content}`);
	}
	Deno.exit(1);
}

console.log("i18n-check: passed");
