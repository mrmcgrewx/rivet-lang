import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { isAbsolute } from "node:path";

const controlHeaders = { "Content-Type": "application/json", "X-Rivet-Request": "1" };
const fixture = name => readFile(new URL(`../../examples/${name}.rvb`, import.meta.url));
const u16 = value => { const bytes = Buffer.alloc(2); bytes.writeUInt16LE(value); return bytes; };
const i64 = value => { const bytes = Buffer.alloc(8); bytes.writeBigInt64LE(BigInt(value)); return bytes; };
const record = (op, type, payload) => Buffer.concat([Buffer.from([op, type]), u16(payload.length), payload]);
function program(records, kind, entry) {
  const body = Buffer.concat([...records, entry]);
  const header = Buffer.alloc(16);
  header.set([0x52, 0x56, 0x42, 0]);
  header.writeUInt16LE(1, 4);
  header.writeUInt16LE(records.length, 8);
  header[10] = kind;
  header.writeUInt32LE(body.length, 12);
  return Buffer.concat([header, body]);
}
const textRecord = text => record(1, 1, Buffer.from(text));
function intServer(value) {
  const key = Buffer.from("max");
  const object = record(0x20, 4, Buffer.concat([u16(1), u16(key.length), key, u16(0)]));
  const path = Buffer.from("/max");
  return program([record(2, 2, i64(value)), object], 2,
    Buffer.concat([u16(0), u16(1), u16(path.length), path, u16(200), u16(1)]));
}
async function downloadBytes(page, name) {
  const pending = page.waitForEvent("download");
  await page.getByRole("button", { name, exact: true }).click();
  const download = await pending;
  const chunks = [];
  for await (const chunk of await download.createReadStream()) chunks.push(chunk);
  return { bytes: Buffer.concat(chunks), filename: download.suggestedFilename() };
}
async function openBytes(page, bytes, filename = "custom.rvb") {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Open binary", exact: true }).click();
  await (await chooser).setFiles({ name: filename, mimeType: "application/octet-stream", buffer: bytes });
  await expect(page.locator("#program-name")).toHaveText(filename);
  await expect(page.locator("#build")).toBeEnabled();
}
async function hexBytes(page, bytes) {
  await page.getByRole("combobox", { name: "Source view", exact: true }).selectOption("hex");
  await page.getByRole("textbox", { name: "Binary source bytes", exact: true }).fill(bytes.toString("hex").match(/../g)?.join(" ") || "");
  await expect(page.locator("#build")).toBeEnabled();
}
async function editLiteral(page, index, value) {
  await page.getByRole("combobox", { name: "Literal instruction" }).selectOption(String(index));
  await page.getByRole("textbox", { name: "Literal value", exact: true }).fill(value);
  await page.getByRole("button", { name: "Apply value", exact: true }).click();
  await expect(page.locator("#build")).toBeEnabled();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
}
async function build(page) {
  const pending = page.waitForResponse(response => response.url().endsWith("/api/build") && response.request().method() === "POST");
  await page.locator("#build").click();
  await expect(page.locator("#source-badge")).toHaveText("Built");
  await expect(page.locator("#build")).toBeEnabled();
  const response = await pending;
  expect(response.ok()).toBeTruthy();
  expect(response.request().headers()["content-type"]).toBe("application/octet-stream");
  expect(response.request().postDataBuffer().subarray(0, 4)).toEqual(Buffer.from([0x52, 0x56, 0x42, 0]));
  return (await response.json()).build;
}
async function startServer(page) {
  await page.getByRole("button", { name: "HTTP server", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await page.getByRole("spinbutton", { name: "Server port" }).fill("0");
  await page.getByRole("button", { name: "Apply port" }).click();
  await expect(page.locator("#build")).toBeEnabled();
  await build(page);
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
}
async function request(page, path = "/hello") {
  await page.getByRole("textbox", { name: "Request path" }).fill(path);
  await page.getByRole("button", { name: "Send request" }).click();
  await expect(page.getByRole("button", { name: "Send request" })).toBeEnabled();
}

test.beforeEach(async ({ page, request }) => {
  expect((await request.post("/api/stop", { headers: controlHeaders, data: {} })).ok()).toBeTruthy();
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Hello world", exact: true })).toBeVisible();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await expect(page.locator("#build")).toBeEnabled();
});
test.afterEach(async ({ request }) => {
  await request.post("/api/stop", { headers: controlHeaders, data: {} });
});

test("binary source has decoded and byte views; source download is the original bytes", async ({ page }) => {
  const bytes = await fixture("hello");
  const decoded = page.getByRole("textbox", { name: "Decoded program", exact: true });
  await expect(decoded).toHaveAttribute("readonly", "");
  await expect(decoded).toHaveValue(/concat/);
  await expect(page.locator("#source-stats")).toContainText(`${bytes.length} bytes`);
  const downloaded = await downloadBytes(page, "Download binary source");
  expect(downloaded.filename).toBe("hello.rvb");
  expect(downloaded.bytes).toEqual(bytes);
  expect(downloaded.bytes[3]).toBe(0);
  expect(downloaded.bytes.toString()).not.toContain('"instructions"');
  await page.getByRole("combobox", { name: "Source view" }).selectOption("hex");
  const hex = page.getByRole("textbox", { name: "Binary source bytes", exact: true });
  expect(Buffer.from((await hex.inputValue()).replace(/\s/g, ""), "hex")).toEqual(bytes);
  await expect(hex).not.toHaveAttribute("readonly", "");
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  const narrowRows = Array.from({ length: Math.ceil(bytes.length / 8) }, (_, index) =>
    bytes.subarray(index * 8, index * 8 + 8).toString("hex").match(/../g).join(" ")).join("\n");
  await hex.fill(narrowRows);
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  expect((await page.locator("#line-numbers").textContent()).split("\n").slice(0, 3)).toEqual(["0000", "0008", "0010"]);
});

test("native Hello and literal edits use raw binary builds and require rebuilding", async ({ page }) => {
  await expect(page.getByRole("button", { name: "Run executable", exact: true })).toBeDisabled();
  await build(page);
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, world!\n");
  await expect(page.locator("#console-detail")).toContainText("Exit code 0");
  await editLiteral(page, 1, "Rivet ☃!");
  await expect(page.locator("#source-badge")).toHaveText("Modified");
  await expect(page.getByRole("button", { name: "Run executable", exact: true })).toBeDisabled();
  await expect(page.locator("#artifact-state")).toContainText("Previous source");
  const changed = (await downloadBytes(page, "Download binary source")).bytes;
  expect(changed.readUInt32LE(12)).toBe(changed.length - 16);
  expect(changed).not.toEqual(await fixture("hello"));
  expect(changed.toString()).toContain("Rivet ☃!");
  await build(page);
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, Rivet ☃!\n");
});

test("artifact metadata, generated code and executable download remain distinct from binary source", async ({ page }) => {
  const compiled = await build(page);
  expect(isAbsolute(compiled.artifact.path)).toBeTruthy();
  await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
  await expect(page.locator("#artifact-size")).toHaveText(`${compiled.artifact.size_bytes.toLocaleString()} bytes`);
  await expect(page.locator("#artifact-profile")).toHaveText("size");
  await expect(page.locator("#artifact-target")).toHaveText(compiled.artifact.target);
  await page.getByRole("tab", { name: "Compiled code", exact: true }).click();
  for (const [value, label] of [["ir", "Compiled intermediate representation"], ["llvm_ir", "LLVM intermediate representation"], ["assembly", "Generated assembly"]]) {
    await page.getByRole("combobox", { name: "Compiled code view" }).selectOption(value);
    await expect(page.getByLabel(label, { exact: true })).toHaveText(compiled[value]);
  }
  const executable = await downloadBytes(page, "Download executable");
  expect(executable.filename).toBe("rivet-program");
  expect(executable.bytes).toEqual(await readFile(compiled.artifact.path));
  expect(executable.bytes.subarray(0, 4)).toEqual(Buffer.from(process.platform === "darwin" ? [0xcf, 0xfa, 0xed, 0xfe] : [0x7f, 0x45, 0x4c, 0x46]));
  expect(executable.bytes).not.toEqual((await downloadBytes(page, "Download binary source")).bytes);
});

test("hex edits and binary decoder errors are visible, and JSON is rejected as source", async ({ page }) => {
  await page.getByRole("combobox", { name: "Source view" }).selectOption("hex");
  await page.getByRole("textbox", { name: "Binary source bytes" }).fill("52 5");
  await expect(page.locator("#diagnostics")).toContainText("two hexadecimal digits");
  await expect(page.locator("#build")).toBeDisabled();
  await expect(page.getByRole("button", { name: "Download binary source", exact: true })).toBeDisabled();
  const bad = Buffer.from(await fixture("hello"));
  bad[0] = 0;
  await hexBytes(page, bad);
  await expect(page.locator("#diagnostics")).toContainText(/magic/i);
  await page.locator("#build").click();
  await expect(page.locator("#diagnostics")).toContainText(/byte|offset/i);
  await expect(page.getByRole("button", { name: "Run executable", exact: true })).toBeDisabled();
  await openBytes(page, Buffer.from('{"version":1,"instructions":[]}'), "not-binary.json");
  await expect(page.locator("#inspection-status")).toHaveText("Invalid binary");
  await expect(page.locator("#diagnostics")).toContainText(/magic|binary/i);
  await page.getByRole("button", { name: "Hello world", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  const changed = Buffer.from(await fixture("hello"));
  // Same-size payload edit authored directly as source bytes: capital W.
  changed[31] = "W".charCodeAt(0);
  await hexBytes(page, changed);
  await build(page);
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, World!\n");
});

test("native checked overflow remains a runtime error for well-formed binary source", async ({ page }) => {
  const bytes = program([
    record(2, 2, i64(9223372036854775807n)), record(2, 2, i64(1)),
    record(0x11, 2, Buffer.concat([u16(0), u16(1)])), record(0x12, 1, u16(2)),
  ], 1, u16(3));
  await openBytes(page, bytes, "overflow.rvb");
  await build(page);
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.locator("#diagnostics")).toContainText(/overflow/i);
  await page.getByRole("tab", { name: "Console", exact: true }).click();
  await expect(page.locator("#console-detail")).toContainText(/Exit code [1-9]/);
});

test("binary HTTP program serves repeated real requests and stops its native process", async ({ page, request: api }) => {
  await startServer(page);
  const address = await page.locator("#server-address").textContent();
  await expect(page.locator("#server-pid")).toHaveText(/^[1-9]\d*$/);
  for (let count = 1; count <= 3; count++) {
    await request(page);
    await expect(page.locator("#response-meta")).toContainText("200");
    await expect(page.locator("#response-meta")).toContainText("application/json");
    await expect(page.getByLabel("HTTP response body")).toHaveText('{"message":"hello world"}');
    await expect(page.locator("#request-count")).toHaveText(String(count));
  }
  await request(page, "/missing");
  await expect(page.locator("#response-meta")).toContainText("404");
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Stopped");
  await expect(page.locator("#server-pid")).toHaveText("—");
  await expect(api.get(`${address}/hello`, { timeout: 1500 })).rejects.toThrow();
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page);
  await expect(page.locator("#request-count")).toHaveText("1");
});

test("running artifact survives invalid binary while edited source takes effect after restart", async ({ page }) => {
  await startServer(page);
  const buildId = await page.locator("#server-build").textContent();
  await hexBytes(page, Buffer.from([0, 1, 2]));
  await page.locator("#build").click();
  await expect(page.locator("#diagnostics")).not.toBeEmpty();
  await expect(page.getByRole("button", { name: "Download executable", exact: true })).toBeDisabled();
  await expect(page.locator("#server-build")).toHaveText(buildId);
  await expect(page.getByRole("button", { name: "Stop", exact: true })).toBeEnabled();
  await request(page);
  await expect(page.getByLabel("HTTP response body")).toContainText("hello world");
  await expect(page.locator("#diagnostics")).not.toBeEmpty();
  await page.getByRole("button", { name: "HTTP server", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await editLiteral(page, 0, "edited binary source");
  await build(page);
  await expect(page.getByRole("button", { name: "Start server", exact: true })).toBeDisabled();
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page);
  await expect(page.getByLabel("HTTP response body")).toHaveText('{"message":"edited binary source"}');
});

test("computed binary example supports integer and boolean byte edits", async ({ page }) => {
  await page.getByRole("button", { name: "Computed values", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await editLiteral(page, 0, "41");
  await editLiteral(page, 6, "false");
  await build(page);
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page, "/computed");
  expect(JSON.parse(await page.getByLabel("HTTP response body").textContent())).toEqual({ message: "The answer is 43", details: { answer: 43, ready: false } });
});

test("Two Sum keeps binary source separate from runtime data and reuses one native build", async ({ page }) => {
  const builds = [];
  page.on("request", request => {
    if (request.url().endsWith("/api/build")) builds.push(request);
  });
  await expect(page.getByRole("textbox", { name: "Runtime input", exact: true })).toBeHidden();
  await page.getByRole("button", { name: "Two Sum", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await expect(page.locator("#source-stats")).toContainText("RVB2");
  await expect(page.locator("#source-help")).toContainText("RVB2");
  const input = page.getByRole("textbox", { name: "Runtime input", exact: true });
  await expect(input).toHaveValue("4 2 7 11 15 9\n");
  await expect(page.locator("#two-sum-input-help")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Decoded program", exact: true })).toHaveAttribute("readonly", "");
  const bytes = await fixture("two-sum");
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  const compiled = await build(page);
  for (const [data, answer] of [["4 2 7 11 15 9\n", "[0,1]\n"], ["3 3 2 4 6\n", "[1,2]\n"], ["2 3 3 6\n", "[0,1]\n"], ["2 9007199254740993 1 9007199254740994\n", "[0,1]\n"]]) {
    await input.fill(data);
    await expect(page.locator("#source-badge")).toHaveText("Built");
    await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
    const pending = page.waitForRequest(request => request.url().endsWith("/api/run"));
    await page.getByRole("button", { name: "Run executable", exact: true }).click();
    expect((await pending).postDataJSON()).toEqual({ build_id: compiled.id, input: data });
    await expect(page.getByLabel("Program output")).toHaveText(answer);
    await expect(page.locator("#console-detail")).toContainText(`Build #${compiled.id}`);
    await expect(page.locator("#console-detail")).toContainText("Exit code 0");
    await expect(input).toBeEnabled();
  }
  await input.fill("3 -3 4 3 0\n");
  await expect(page.locator("#runtime-input-status")).toContainText("Input changed");
  expect(builds).toHaveLength(1);
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  await page.getByRole("combobox", { name: "Source view", exact: true }).selectOption("hex");
  const hex = page.getByRole("textbox", { name: "Binary source bytes", exact: true });
  expect(Buffer.from((await hex.inputValue()).replace(/\s/g, ""), "hex")).toEqual(bytes);
  await page.getByRole("button", { name: "Hello world", exact: true }).click();
  await expect(input).toBeHidden();
  await build(page);
  const pending = page.waitForRequest(request => request.url().endsWith("/api/run"));
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  expect((await pending).postDataJSON()).not.toHaveProperty("input");
  await expect(page.getByLabel("Program output")).toHaveText("Hello, world!\n");
});

test("compute runtime input errors recover without rebuilding or modifying binary source", async ({ page }) => {
  await page.getByRole("button", { name: "Two Sum", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  const compiled = await build(page);
  const input = page.getByRole("textbox", { name: "Runtime input", exact: true });
  await input.fill("2 3 invalid 6");
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.locator("#diagnostics")).toContainText(/integer|input|decimal/i);
  await expect(page.locator("#source-badge")).toHaveText("Built");
  await expect(input).toBeEnabled();
  await input.fill("2 3 3 6\n");
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("[0,1]\n");
  await expect(page.locator("#diagnostics li")).toHaveCount(0);
  await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
  // Count actual UTF-8 bytes, including data that is not valid numeric input.
  await input.fill("é".repeat(131073));
  await expect(input).toHaveAttribute("aria-invalid", "true");
  await expect(page.locator("#runtime-input-status")).toContainText("too large");
  await expect(page.getByRole("button", { name: "Run executable", exact: true })).toBeDisabled();
  await input.fill("2 3 3 6\n");
  await expect(input).toHaveAttribute("aria-invalid", "false");
  await expect(page.getByRole("button", { name: "Run executable", exact: true })).toBeEnabled();
  const bytes = await fixture("two-sum");
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  await openBytes(page, bytes, "imported-solution.rvb");
  await expect(input).toHaveValue("");
  await expect(input).toBeVisible();
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(input).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
});

test("Add Two Numbers uses two digit lists across published and hundred-digit carry cases", async ({ page }) => {
  const builds = [];
  page.on("request", request => {
    if (request.url().endsWith("/api/build")) builds.push(request);
  });
  await page.getByRole("button", { name: "Add Two Numbers", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  const input = page.getByRole("textbox", { name: "Runtime input", exact: true });
  await expect(input).toHaveValue("3 2 4 3 3 5 6 4\n");
  await expect(page.locator("#add-two-numbers-input-help")).toBeVisible();
  await expect(page.locator("#add-two-numbers-input-help")).toContainText("ones place");
  await expect(page.locator("#two-sum-input-help")).toBeHidden();
  await expect(input).toHaveAttribute("aria-describedby", "runtime-input-help add-two-numbers-input-help");
  const bytes = await fixture("add-two-numbers");
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  const compiled = await build(page);
  const cases = [
    ["3 2 4 3 3 5 6 4\n", [7, 0, 8]],
    ["1 0 1 0\n", [0]],
    ["7 9 9 9 9 9 9 9 4 9 9 9 9\n", [8, 9, 9, 9, 0, 0, 0, 1]],
    [`100 ${Array(100).fill(9).join(" ")} 1 1\n`, [...Array(100).fill(0), 1]],
  ];
  for (const [data, answer] of cases) {
    await input.fill(data);
    await expect(page.locator("#source-badge")).toHaveText("Built");
    await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
    const pending = page.waitForRequest(request => request.url().endsWith("/api/run"));
    await page.getByRole("button", { name: "Run executable", exact: true }).click();
    expect((await pending).postDataJSON()).toEqual({ build_id: compiled.id, input: data });
    await expect(page.getByLabel("Program output")).toHaveText(`${JSON.stringify(answer)}\n`);
    await expect(page.locator("#console-detail")).toContainText(`Build #${compiled.id}`);
    await expect(page.locator("#console-detail")).toContainText("Exit code 0");
    await expect(input).toBeEnabled();
  }
  expect(builds).toHaveLength(1);
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  await page.getByRole("combobox", { name: "Source view", exact: true }).selectOption("hex");
  const hex = page.getByRole("textbox", { name: "Binary source bytes", exact: true });
  expect(Buffer.from((await hex.inputValue()).replace(/\s/g, ""), "hex")).toEqual(bytes);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator("#add-two-numbers-input-help")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
});

test("Add Two Numbers input assertions recover on the same build and retain other example guidance", async ({ page }) => {
  await page.getByRole("button", { name: "Add Two Numbers", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  const input = page.getByRole("textbox", { name: "Runtime input", exact: true });
  const compiled = await build(page);
  await input.fill("1 12 1 0\n");
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.locator("#diagnostics")).toContainText("assertion failed");
  await expect(page.locator("#source-badge")).toHaveText("Built");
  await expect(input).toBeEnabled();
  await input.fill("3 2 4 3 3 5 6 4\n");
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("[7,0,8]\n");
  await expect(page.locator("#diagnostics li")).toHaveCount(0);
  await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(await fixture("add-two-numbers"));
  await page.getByRole("button", { name: "Two Sum", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await expect(input).toHaveValue("4 2 7 11 15 9\n");
  await expect(page.locator("#two-sum-input-help")).toBeVisible();
  await expect(page.locator("#add-two-numbers-input-help")).toBeHidden();
  await expect(input).toHaveAttribute("aria-describedby", "runtime-input-help two-sum-input-help");
});

test("Median uses one native binary for odd fractional negative and empty-sided inputs", async ({ page }) => {
  const builds = [];
  page.on("request", request => {
    if (request.url().endsWith("/api/build")) builds.push(request);
  });
  await page.getByRole("button", { name: "Median of Two Sorted Arrays", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  const input = page.getByRole("textbox", { name: "Runtime input", exact: true });
  await expect(input).toHaveValue("2 1 3 1 2\n");
  await expect(page.locator("#median-input-help")).toBeVisible();
  await expect(page.locator("#median-input-help")).toContainText("already sorted");
  await expect(page.locator("#median-input-help")).toContainText("caller preconditions");
  await expect(page.locator("#median-input-help")).toContainText("O(log(min(m,n)+1))");
  await expect(page.locator("#median-input-help")).toContainText("O(m+n)");
  await expect(input).toHaveAttribute("aria-describedby", "runtime-input-help median-input-help");
  await expect(page.getByRole("textbox", { name: "Decoded program", exact: true })).toHaveValue(/Float/);
  const bytes = await fixture("median-two-sorted-arrays");
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  const compiled = await build(page);
  const cases = [
    ["2 1 3 1 2\n", "2.0000000000000000e+00\n"],
    ["2 1 2 2 3 4\n", "2.5000000000000000e+00\n"],
    ["1 -1 1 0\n", "-5.0000000000000000e-01\n"],
    ["0 2 2 3\n", "2.5000000000000000e+00\n"],
    ["2 -3 -2 0\n", "-2.5000000000000000e+00\n"],
  ];
  for (const [data, answer] of cases) {
    await input.fill(data);
    await expect(page.locator("#source-badge")).toHaveText("Built");
    await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
    const pending = page.waitForRequest(request => request.url().endsWith("/api/run"));
    await page.getByRole("button", { name: "Run executable", exact: true }).click();
    expect((await pending).postDataJSON()).toEqual({ build_id: compiled.id, input: data });
    await expect(page.getByLabel("Program output")).toHaveText(answer);
    expect(await page.getByLabel("Program output").textContent()).toBe(answer);
    await expect(page.locator("#console-detail")).toContainText(`Build #${compiled.id}`);
    await expect(page.locator("#console-detail")).toContainText("Exit code 0");
    await expect(input).toBeEnabled();
  }
  expect(builds).toHaveLength(1);
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  await page.getByRole("combobox", { name: "Source view", exact: true }).selectOption("hex");
  const hex = page.getByRole("textbox", { name: "Binary source bytes", exact: true });
  expect(Buffer.from((await hex.inputValue()).replace(/\s/g, ""), "hex")).toEqual(bytes);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator("#median-input-help")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
});

test("Median input failures recover without rebuilding and other example guidance is retained", async ({ page }) => {
  await page.getByRole("button", { name: "Median of Two Sorted Arrays", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  const input = page.getByRole("textbox", { name: "Runtime input", exact: true });
  const compiled = await build(page);
  for (const [invalid, reason] of [["0 0\n", /assertion failed/], ["2 1 invalid 1 2\n", /integer|input|decimal/i]]) {
    await input.fill(invalid);
    await page.getByRole("button", { name: "Run executable", exact: true }).click();
    await expect(page.locator("#diagnostics")).toContainText(reason);
    await expect(page.locator("#source-badge")).toHaveText("Built");
    await expect(input).toBeEnabled();
    await input.fill("2 1 2 2 3 4\n");
    await page.getByRole("button", { name: "Run executable", exact: true }).click();
    await expect(page.getByLabel("Program output")).toHaveText("2.5000000000000000e+00\n");
    await expect(page.locator("#diagnostics li")).toHaveCount(0);
    await expect(page.getByLabel("Executable path")).toHaveText(compiled.artifact.path);
  }
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(await fixture("median-two-sorted-arrays"));
  await page.getByRole("button", { name: "Add Two Numbers", exact: true }).click();
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await expect(input).toHaveValue("3 2 4 3 3 5 6 4\n");
  await expect(page.locator("#add-two-numbers-input-help")).toBeVisible();
  await expect(page.locator("#median-input-help")).toBeHidden();
  await expect(input).toHaveAttribute("aria-describedby", "runtime-input-help add-two-numbers-input-help");
});

test("literal editor preserves exact i64 values in bytes and HTTP output", async ({ page }) => {
  await openBytes(page, intServer(0), "integer.rvb");
  await editLiteral(page, 0, "9223372036854775807");
  let downloaded = await downloadBytes(page, "Download binary source");
  expect(downloaded.bytes.readBigInt64LE(20)).toBe(9223372036854775807n);
  await build(page);
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page, "/max");
  await expect(page.getByLabel("HTTP response body")).toHaveText('{"max":9223372036854775807}');
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await editLiteral(page, 0, "-9223372036854775808");
  downloaded = await downloadBytes(page, "Download binary source");
  expect(downloaded.bytes.readBigInt64LE(20)).toBe(-9223372036854775808n);
  await page.getByRole("textbox", { name: "Literal value" }).fill("9223372036854775808");
  await page.getByRole("button", { name: "Apply value" }).click();
  await expect(page.locator("#diagnostics")).toContainText("outside the signed 64-bit range");
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(downloaded.bytes);
});

test("new and opened binary files roundtrip exactly and decoded display is never saved as source", async ({ page }) => {
  await page.getByRole("button", { name: "New binary source", exact: true }).click();
  await expect(page.locator("#program-name")).toHaveText("untitled.rvb");
  await expect(page.locator("#inspection-status")).toContainText("Valid binary");
  await editLiteral(page, 0, "Authored as bytes");
  const created = await downloadBytes(page, "Download binary source");
  expect(created.filename).toBe("untitled.rvb");
  expect(created.bytes).toEqual(program([textRecord("Authored as bytes")], 1, u16(0)));
  const bytes = program([textRecord("Opened from disk ☃")], 1, u16(0));
  await openBytes(page, bytes, "roundtrip.rvb");
  await expect(page.getByRole("textbox", { name: "Decoded program", exact: true })).toHaveValue(/Opened from disk/);
  expect((await downloadBytes(page, "Download binary source")).bytes).toEqual(bytes);
  await build(page);
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Opened from disk ☃\n");
});

test("native exit diagnostics preserve binary compiler errors and are not duplicated", async ({ page }) => {
  await startServer(page);
  const pid = Number(await page.locator("#server-pid").textContent());
  await hexBytes(page, Buffer.from([0, 1, 2]));
  await page.locator("#build").click();
  await expect(page.locator("#diagnostics")).not.toBeEmpty();
  const compilerError = await page.locator("#diagnostics").textContent();
  process.kill(pid, "SIGTERM");
  await expect(page.locator("#server-badge")).toHaveText("Stopped");
  await expect(page.locator("#diagnostics")).toContainText("Native server exited");
  await expect(page.locator("#diagnostics")).toContainText(compilerError);
  const count = await page.locator("#diagnostics li").count();
  await page.waitForResponse(response => response.url().endsWith("/api/status"));
  await expect(page.locator("#diagnostics li")).toHaveCount(count);
});

test("binary views work with keyboard navigation and compact layout without page errors", async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.getByRole("tab", { name: "Console", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("tab", { name: /Diagnostics/ })).toBeFocused();
  await page.keyboard.press("End");
  await expect(page.getByRole("tab", { name: "Compiled code", exact: true })).toBeFocused();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole("textbox", { name: "Decoded program", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply value", exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await page.getByRole("combobox", { name: "Source view" }).selectOption("hex");
  await expect(page.getByRole("textbox", { name: "Binary source bytes", exact: true })).toBeVisible();
  await build(page);
  await expect(page.getByLabel("Executable path")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await page.getByRole("button", { name: "Run executable", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, world!\n");
  expect(errors).toEqual([]);
});
