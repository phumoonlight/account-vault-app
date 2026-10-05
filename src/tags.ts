// Mirrors normalize_tags in src-tauri/src/vault/model.rs, so the form shows
// exactly what will be saved.

export const MAX_TAGS = 20;
export const MAX_TAG_LEN = 32;

export function normalizeTag(tag: string): string {
  return [...tag.trim().split(/\s+/).join(" ")].slice(0, MAX_TAG_LEN).join("").trimEnd();
}

export const sameTag = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

/** Returns `tags` plus `tag` (normalized), unless it's empty, a duplicate, or over the limit. */
export function addTag(tags: string[], tag: string): string[] {
  const t = normalizeTag(tag);
  if (!t || tags.length >= MAX_TAGS || tags.some((x) => sameTag(x, t))) return tags;
  return [...tags, t];
}

/** Every distinct tag across entries with its usage count, sorted by name. */
export function collectTags(entries: { tags: string[] }[]): { tag: string; count: number }[] {
  const counts = new Map<string, { tag: string; count: number }>();
  for (const e of entries) {
    for (const tag of e.tags) {
      const key = tag.toLowerCase();
      const item = counts.get(key);
      if (item) item.count++;
      else counts.set(key, { tag, count: 1 });
    }
  }
  return [...counts.values()].sort((a, b) =>
    a.tag.localeCompare(b.tag, undefined, { sensitivity: "base" }),
  );
}
