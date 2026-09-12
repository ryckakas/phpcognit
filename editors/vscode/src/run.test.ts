import { strict as assert } from "node:assert";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { scan } from "./run";
import type { Binary } from "./binary";

const scripts = mkdtempSync(join(tmpdir(), "phpcognit-fake-cli-"));
let counter = 0;

/**
 * A stand-in for the CLI that prints whatever the case needs and exits how it
 * likes. It has to be a script file rather than `node -e`, because node reads
 * the trailing `--format json` as its own option and refuses to start.
 */
function fakeCli(script: string): Binary {
  const file = join(scripts, `fake-${(counter += 1)}.js`);
  writeFileSync(file, script);

  return { command: process.execPath, prefixArgs: [file], source: "path" };
}

const report = (findings: unknown[], breaches = findings.length) =>
  JSON.stringify({ threshold: 15, breaches, findings });

function run(binary: Binary) {
  return scan({ binary, file: "any.php", workspaceRoot: process.cwd(), threshold: 15 });
}

test("returns the findings a report contains", async () => {
  const finding = { path: "src/A.php", line: 7, name: "A::tangled", score: 21 };
  const findings = await run(fakeCli(`console.log(${JSON.stringify(report([finding]))})`));

  assert.deepEqual(findings, [finding]);
});

test("a clean report yields no findings rather than an error", async () => {
  const findings = await run(fakeCli(`console.log(${JSON.stringify(report([]))})`));

  assert.deepEqual(findings, []);
});

// The CLI exits 1 whenever anything breaches, which is it working, not failing.
// Treating that as an error would leave every breaching file without diagnostics.
test("a non-zero exit still succeeds when the report parses", async () => {
  const finding = { path: "src/A.php", line: 7, name: "A::tangled", score: 21 };
  const script = `console.log(${JSON.stringify(report([finding]))}); process.exit(1)`;

  assert.deepEqual(await run(fakeCli(script)), [finding]);
});

test("rejects when nothing is written to stdout", async () => {
  await assert.rejects(run(fakeCli("process.exit(1)")));
});

test("rejects rather than throwing on output that is not a report", async () => {
  await assert.rejects(run(fakeCli("console.log('not json at all')")));
});

test("rejects on JSON that is not shaped like a report", async () => {
  await assert.rejects(run(fakeCli(`console.log(JSON.stringify({ unexpected: true }))`)));
});

test("surfaces stderr when the command fails outright", async () => {
  const script = "console.error('grammar exploded'); process.exit(2)";

  await assert.rejects(run(fakeCli(script)), /grammar exploded/);
});
