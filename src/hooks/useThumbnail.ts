import { useEffect, useState } from "react";
import { readFileBase64 } from "../services/commands";

const cache = new Map<string, string>();
const pending = new Map<string, Promise<string | null>>();

function mimeFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "png":
      return "image/png";
    case "webp":
      return "image/webp";
    case "gif":
      return "image/gif";
    case "bmp":
      return "image/bmp";
    default:
      return "image/jpeg";
  }
}

async function loadThumb(relativePath: string): Promise<string | null> {
  try {
    const b64 = await readFileBase64(relativePath);
    const mime = mimeFromPath(relativePath);
    const dataUrl = `data:${mime};base64,${b64}`;
    cache.set(relativePath, dataUrl);
    return dataUrl;
  } catch {
    return null;
  } finally {
    pending.delete(relativePath);
  }
}

export function useThumbnail(relativePath: string | null | undefined): string | null {
  const [url, setUrl] = useState<string | null>(
    relativePath ? cache.get(relativePath) ?? null : null
  );

  useEffect(() => {
    if (!relativePath) {
      setUrl(null);
      return;
    }

    const cached = cache.get(relativePath);
    if (cached) {
      setUrl(cached);
      return;
    }

    let cancelled = false;

    let promise = pending.get(relativePath);
    if (!promise) {
      promise = loadThumb(relativePath);
      pending.set(relativePath, promise);
    }

    promise.then((result) => {
      if (!cancelled) setUrl(result);
    });

    return () => {
      cancelled = true;
    };
  }, [relativePath]);

  return url;
}

export function invalidateThumbnailCache(relativePath?: string) {
  if (relativePath) {
    cache.delete(relativePath);
  } else {
    cache.clear();
  }
}
