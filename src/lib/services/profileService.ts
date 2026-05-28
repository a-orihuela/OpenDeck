/** Organises a flat list of profile IDs into folder buckets.
 *  Profiles with a "/" prefix go into a named folder; the rest go in "". */
export function organizeIntoFolders(profiles: string[]): Record<string, string[]> {
	const folders: Record<string, string[]> = {};
	for (const id of profiles) {
		const folder = id.includes("/") ? id.split("/")[0] : "";
		if (folders[folder]) folders[folder].push(id);
		else folders[folder] = [id];
	}
	return folders;
}

/** Generates a non-colliding duplicate name for a profile.
 *  Appends " Copy", then " Copy 2", " Copy 3", etc. */
export function generateDuplicateName(id: string, existingIds: string[]): string {
	let candidate = id + " Copy";
	let counter = 1;
	while (existingIds.includes(candidate)) {
		counter++;
		candidate = `${id} Copy ${counter}`;
	}
	return candidate;
}

/** Returns a new folders map after renaming oldId → newId. */
export function updateFoldersAfterRename(
	folders: Record<string, string[]>,
	oldId: string,
	newId: string,
): Record<string, string[]> {
	const next = structuredClone(folders) as Record<string, string[]>;
	const oldFolder = oldId.includes("/") ? oldId.split("/")[0] : "";
	const newFolder = newId.includes("/") ? newId.split("/")[0] : "";

	if (next[oldFolder]) {
		const idx = next[oldFolder].indexOf(oldId);
		if (idx !== -1) {
			next[oldFolder].splice(idx, 1);
			if (next[oldFolder].length === 0 && oldFolder !== "") delete next[oldFolder];
		}
	}
	if (next[newFolder]) next[newFolder].push(newId);
	else next[newFolder] = [newId];

	return next;
}

/** Returns a new folders map after removing a profile id. */
export function removeFolderEntry(
	folders: Record<string, string[]>,
	id: string,
): Record<string, string[]> {
	const next = structuredClone(folders) as Record<string, string[]>;
	const folder = id.includes("/") ? id.split("/")[0] : "";
	if (!next[folder]) return next;
	const idx = next[folder].indexOf(id);
	if (idx !== -1) next[folder].splice(idx, 1);
	return next;
}

/** Returns all profile ids across all folders as a flat list. */
export function flatProfileList(folders: Record<string, string[]>): string[] {
	return Object.values(folders).flat();
}
