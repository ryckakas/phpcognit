import { execFile } from "node:child_process";
import type { Binary } from "./binary";

export interface Finding {
  path: string;
  line: number;
  name: string;
  score: number;
}

interface Report {
  threshold: number;
  breaches: number;
  findings: Finding[];
}

export interface ScanOptions {
  binary: Binary;
  file: string;
  workspaceRoot: string;
  threshold: number;
}

/**
 * The baseline file records paths relative to the working directory, so the
 * scan has to run from the workspace root. Running it anywhere else makes every
 * baselined finding reappear as a fresh diagnostic.
 *
 * `--all` is deliberately not passed: the editor should show exactly what CI
 * would fail on, baseline and suppressions included.
 */
export function scan(options: ScanOptions): Promise<Finding[]> {
  const { binary, file, workspaceRoot, threshold } = options;
  const args = [...binary.prefixArgs, "--format", "json", "--over", String(threshold), file];

  return new Promise((resolve, reject) => {
    execFile(binary.command, args, { cwd: workspaceRoot }, (error, stdout, stderr) => {
      // A breach exits non-zero, which is the tool working, not failing. Only
      // unparseable output means something actually went wrong.
      const report = parseReport(stdout);

      if (report !== undefined) {
        resolve(report.findings);
        return;
      }

      reject(new Error(stderr.trim() !== "" ? stderr.trim() : (error?.message ?? "no output")));
    });
  });
}

function parseReport(stdout: string): Report | undefined {
  if (stdout.trim() === "") {
    return undefined;
  }

  try {
    const parsed: unknown = JSON.parse(stdout);
    return isReport(parsed) ? parsed : undefined;
  } catch {
    return undefined;
  }
}

function isReport(value: unknown): value is Report {
  return (
    typeof value === "object" &&
    value !== null &&
    Array.isArray((value as Report).findings)
  );
}
