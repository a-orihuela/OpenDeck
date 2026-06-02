type IconSpec = {
	phosphor: string;
	weight?: string;
	notes?: string;
	secondaryOpacity?: number;
};

type IconMap = {
	style?: string;
	source?: string;
	icons: Record<string, IconSpec>;
	generation?: {
		targetSize?: number;
		primaryOpacity?: number;
		secondaryOpacity?: number;
		fill?: string;
		preserveViewBox?: string;
	};
};

const MAP_PATH = new URL("./phosphor-icon-map.json", import.meta.url);
const BUILTIN_DIR = new URL("../static/builtin/", import.meta.url);
const DEFAULT_CORE_VERSION = "2.1.1";

function toKebabCase(name: string): string {
	return name
		.replace(/([a-z\d])([A-Z])/g, "$1-$2")
		.replace(/([A-Z]+)([A-Z][a-z])/g, "$1-$2")
		.toLowerCase();
}

function normaliseSvg(svg: string, cfg: Required<NonNullable<IconMap["generation"]>>, iconSecondaryOpacity?: number): string {
	let out = svg.trim();

	out = out.replace(/<\?xml[^>]*>\s*/g, "");
	out = out.replace(/<!DOCTYPE[^>]*>\s*/g, "");

	out = out.replace(/<svg\s+([^>]*?)>/, (_m, attrs: string) => {
		const cleaned = attrs
			.replace(/\swidth="[^"]*"/g, "")
			.replace(/\sheight="[^"]*"/g, "")
			.replace(/\sfill="[^"]*"/g, "")
			.replace(/xmlns="[^"]*"/g, "")
			.trim();

		const withViewBox = /\bviewBox=/.test(cleaned)
			? cleaned
			: `${cleaned} viewBox="${cfg.preserveViewBox}"`;

		return `<svg ${withViewBox} width="${cfg.targetSize}" height="${cfg.targetSize}" fill="${cfg.fill}" xmlns="http://www.w3.org/2000/svg">`;
	});

	out = out.replace(/fill="(?:#000|#000000|black)"/gi, `fill="${cfg.fill}"`);
	const secondaryOpacity = iconSecondaryOpacity ?? cfg.secondaryOpacity;
	out = out.replace(/opacity="0\.2"/g, `opacity="${secondaryOpacity}"`);

	return `${out}\n`;
}

async function fetchSvg(icon: string, weight: string, coreVersion: string): Promise<string> {
	const iconName = toKebabCase(icon);
	const url = `https://unpkg.com/@phosphor-icons/core@${coreVersion}/assets/${weight}/${iconName}-${weight}.svg`;
	const response = await fetch(url);
	if (!response.ok) {
		throw new Error(`Failed to fetch ${url}: ${response.status}`);
	}
	return await response.text();
}

function parseArgs() {
	const args = new Set(Deno.args);
	const getValue = (prefix: string): string | null => {
		for (const arg of Deno.args) {
			if (arg.startsWith(`${prefix}=`)) return arg.slice(prefix.length + 1);
		}
		return null;
	};

	return {
		dryRun: args.has("--dry-run"),
		only: getValue("--only"),
		coreVersion: getValue("--core-version") ?? DEFAULT_CORE_VERSION,
	};
}

const { dryRun, only, coreVersion } = parseArgs();
const rawMap = await Deno.readTextFile(MAP_PATH);
const map = JSON.parse(rawMap) as IconMap;

if (!map.icons || Object.keys(map.icons).length === 0) {
	throw new Error("Icon map is empty.");
}

const generation = {
	targetSize: map.generation?.targetSize ?? 256,
	primaryOpacity: map.generation?.primaryOpacity ?? 1,
	secondaryOpacity: map.generation?.secondaryOpacity ?? 0.2,
	fill: map.generation?.fill ?? "currentColor",
	preserveViewBox: map.generation?.preserveViewBox ?? "0 0 256 256",
};

const selected = only
	? Object.entries(map.icons).filter(([file]) => file === only)
	: Object.entries(map.icons);

if (selected.length === 0) {
	throw new Error(`No icon matched --only=${only}`);
}

for (const [targetFile, spec] of selected) {
	const weight = spec.weight ?? map.style ?? "duotone";
	const rawSvg = await fetchSvg(spec.phosphor, weight, coreVersion);
	const normalised = normaliseSvg(rawSvg, generation, spec.secondaryOpacity);
	const outputPath = new URL(targetFile, BUILTIN_DIR);

	if (dryRun) {
		console.log(`[dry-run] ${targetFile} <= ${spec.phosphor} (${weight})`);
		continue;
	}

	await Deno.writeTextFile(outputPath, normalised);
	console.log(`updated ${targetFile} <= ${spec.phosphor} (${weight})`);
}
