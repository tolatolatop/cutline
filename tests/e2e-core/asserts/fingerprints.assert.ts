import * as fs from "node:fs";
import * as crypto from "node:crypto";

export interface Fingerprint {
  algo: string;
  value: string;
  basis: string;
}

/** Compute SHA-256 hash of a file (matches Cutline's fingerprint logic). */
export function computeSha256(filePath: string): string {
  const data = fs.readFileSync(filePath);
  return crypto.createHash("sha256").update(data).digest("hex");
}

/**
 * Assert that a fingerprint object matches the SHA-256 of the source file.
 */
export function assertFingerprintMatches(
  fingerprint: Fingerprint,
  sourceFilePath: string
): void {
  if (fingerprint.algo !== "sha256") {
    throw new Error(
      `Unexpected fingerprint algorithm: "${fingerprint.algo}" (expected "sha256")`
    );
  }

  const expected = computeSha256(sourceFilePath);
  if (fingerprint.value !== expected) {
    throw new Error(
      `Fingerprint mismatch for ${sourceFilePath}:\n` +
        `  expected: ${expected}\n` +
        `  actual:   ${fingerprint.value}`
    );
  }
}

/**
 * Assert that all fingerprints in an asset array are unique
 * (no duplicate imports).
 */
export function assertUniqueFingerprints(
  assets: { fingerprint: Fingerprint }[]
): void {
  const seen = new Set<string>();
  for (const asset of assets) {
    const key = `${asset.fingerprint.algo}:${asset.fingerprint.value}`;
    if (seen.has(key)) {
      throw new Error(`Duplicate fingerprint found: ${key}`);
    }
    seen.add(key);
  }
}
