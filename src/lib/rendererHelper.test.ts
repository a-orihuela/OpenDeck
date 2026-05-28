import { describe, it, expect, vi } from "vitest";

vi.mock("./ports.ts", () => ({
	getWebserverUrl: (path: string) => `http://localhost:57118/${path}`,
}));

vi.mock("./api/commands.ts", () => ({
	updateImage: vi.fn(),
}));

import { getImage } from "./rendererHelper.ts";

describe("getImage", () => {
	it("returns /alert.png when image is undefined", () => {
		expect(getImage(undefined, undefined)).toBe("/alert.png");
	});

	it("returns /alert.png when image is empty string", () => {
		expect(getImage("", undefined)).toBe("/alert.png");
	});

	it("strips opendeck/ prefix and returns path without prefix", () => {
		expect(getImage("opendeck/multi-action.png", undefined)).toBe("/multi-action.png");
	});

	it("routes non-data URLs through getWebserverUrl", () => {
		expect(getImage("/path/to/icon.png", undefined)).toBe("http://localhost:57118//path/to/icon.png");
	});

	it("passes through valid base64 image unchanged", () => {
		const base64 = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUA";
		expect(getImage(base64, undefined)).toBe(base64);
	});

	it("returns fallback for base64 image with empty data segment", () => {
		const noData = "data:image/png;base64,";
		expect(getImage(noData, undefined)).toBe("/alert.png");
	});

	it("returns fallback result when primary image is undefined and fallback is provided", () => {
		const fallback = "opendeck/ok.png";
		expect(getImage(undefined, fallback)).toBe("/ok.png");
	});

	it("normalises a percent-encoded SVG XML data URL", () => {
		const encodedSvg = "data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%2F%3E";
		const result = getImage(encodedSvg, undefined);
		expect(result).toMatch(/^data:image\/svg\+xml,/);
		expect(result).toContain("%3Csvg");
	});
});
