const KEY = 'research-pdf-bookmarks-v1';

type BookmarkStore = Record<string, number[]>;

function readStore(): BookmarkStore {
  try {
    return JSON.parse(localStorage.getItem(KEY) || '{}') as BookmarkStore;
  } catch {
    return {};
  }
}

export function getBookmarks(fingerprint: string): number[] {
  return readStore()[fingerprint] || [];
}

export function toggleBookmark(fingerprint: string, page: number): number[] {
  const store = readStore();
  const set = new Set(store[fingerprint] || []);
  if (set.has(page)) set.delete(page);
  else set.add(page);
  const result = [...set].sort((a, b) => a - b);
  store[fingerprint] = result;
  localStorage.setItem(KEY, JSON.stringify(store));
  return result;
}
