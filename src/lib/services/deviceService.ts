import type { ActionInstance, Context, DeviceInfo } from "../bindings.ts";
import type { CopiedItem } from "../state/appState.ts";
import { createInstance, moveInstance } from "../api/commands.ts";

// ── Grid navigation helpers ───────────────────────────────────────────────────

/** Computes the number of keys in each visual row for a device. */
export function computeGridRowLengths(device: DeviceInfo): number[] {
	return [
		...Array(device.rows).fill(device.columns),
		...(device.encoders > 0 ? [device.encoders] : []),
		...(device.touchpoints > 0 ? [device.touchpoints] : []),
	];
}

export function flatIndexFromRowCol(rowLengths: number[], row: number, col: number): number {
	let index = 0;
	for (let r = 0; r < row; r++) index += rowLengths[r];
	return index + col;
}

export function rowColFromFlatIndex(rowLengths: number[], flatIndex: number): [number, number] {
	let remaining = flatIndex;
	for (let r = 0; r < rowLengths.length; r++) {
		if (remaining < rowLengths[r]) return [r, remaining];
		remaining -= rowLengths[r];
	}
	return [0, 0];
}

// ── Drag-and-drop / paste operations ─────────────────────────────────────────

export type DropResult = { position: number; instance: ActionInstance } | null;

/**
 * Handles dropping an action (from the action list) onto a slot.
 * Returns the new instance or null if the slot was already occupied.
 */
export async function dropNewAction(
	context: Context,
	actionJson: string,
	currentSlot: ActionInstance | null,
): Promise<ActionInstance | null> {
	if (currentSlot) return null;
	const action = JSON.parse(actionJson);
	return createInstance(context, action);
}

/**
 * Handles moving an existing instance from one slot to another (drag from grid).
 * Returns { position: oldPosition, instance: newInstance } on success.
 */
export async function dropMoveInstance(
	device: DeviceInfo,
	profile: { id: string },
	srcController: string,
	srcPosition: number,
	destination: Context,
): Promise<{ instance: ActionInstance; oldPosition: number; oldController: string }> {
	const source: Context = {
		device: device.id,
		profile: profile.id,
		controller: srcController,
		position: srcPosition,
	};
	const instance = await moveInstance(source, destination, false);
	return { instance, oldPosition: srcPosition, oldController: srcController };
}

/**
 * Handles pasting a copied item onto a destination slot.
 * Returns the new instance, or null if nothing was created.
 */
export async function pasteItem(
	item: CopiedItem,
	destination: Context,
	currentSlot: ActionInstance | null,
	activeFolderContext: string | null,
): Promise<ActionInstance | null> {
	if (item.type === "action") {
		if (currentSlot) return null;
		return createInstance(destination, item.action);
	}
	// type === "instance" — copy/move; not allowed inside folders
	if (activeFolderContext) return null;
	return moveInstance(item.source as any, destination, true);
}
