import { strict as assert } from "node:assert";
import { test } from "node:test";
import { resolveBinary } from "./binary";

test("falls back to the bundled package when nothing is configured", async () => {
  const binary = await resolveBinary("");

  assert.ok(binary, "the phpcognit dependency should always be resolvable here");
  assert.equal(binary.source, "bundled");
  assert.equal(binary.command, process.execPath, "the npm package is a Node wrapper");
  assert.match(binary.prefixArgs[0] ?? "", /run-phpcognit\.js$/);
});

test("an explicit path wins over the bundled copy", async () => {
  const binary = await resolveBinary(process.execPath);

  assert.ok(binary);
  assert.equal(binary.source, "setting");
  assert.equal(binary.command, process.execPath);
  assert.deepEqual(binary.prefixArgs, []);
});

// Silently ignoring a broken setting beats refusing to run at all: the user still
// gets diagnostics, and the notification tells them the path was not used.
test("a configured path that does not exist is ignored", async () => {
  const binary = await resolveBinary("/definitely/not/a/binary");

  assert.ok(binary);
  assert.notEqual(binary.source, "setting");
});

test("whitespace is not treated as a configured path", async () => {
  const binary = await resolveBinary("   ");

  assert.ok(binary);
  assert.notEqual(binary.source, "setting");
});
