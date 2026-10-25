import { test, expect } from "@playwright/test";

const apiHeaders = {
  "Content-Type": "application/json",
  "X-Rivet-Request": "1",
};

async function editProgram(page, transform) {
  const editor = page.getByRole("textbox", { name: "Program source" });
  const program = JSON.parse(await editor.inputValue());
  transform(program);
  await editor.fill(JSON.stringify(program, null, 2));
}

async function build(page) {
  await page.locator("#build").click();
  await expect(page.locator("#source-badge")).toHaveText("Built");
  await expect(page.locator("#build")).toBeEnabled();
}

async function startServer(page) {
  await page.getByRole("button", { name: "HTTP server", exact: true }).click();
  await editProgram(page, (program) => {
    program.entry.port = 0;
  });
  await build(page);
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await expect(
    page.getByRole("button", { name: "Stop", exact: true }),
  ).toBeEnabled();
}

async function request(page, path = "/hello") {
  await page.getByRole("textbox", { name: "Request path" }).fill(path);
  await page.getByRole("button", { name: "Send request" }).click();
  await expect(
    page.getByRole("button", { name: "Send request" }),
  ).toBeEnabled();
}

test.beforeEach(async ({ page, request }) => {
  const response = await request.post("/api/stop", {
    headers: apiHeaders,
    data: {},
  });
  expect(response.ok()).toBeTruthy();
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Hello world", exact: true }),
  ).toBeVisible();
  await expect(page.locator("#build")).toBeEnabled();
});

test.afterEach(async ({ request }) => {
  await request.post("/api/stop", { headers: apiHeaders, data: {} });
});

test("Hello world builds typed IR and runs; changed source must rebuild and changes output", async ({
  page,
}) => {
  await expect(
    page.getByRole("button", { name: "Run program", exact: true }),
  ).toBeDisabled();
  await build(page);
  await page.getByRole("tab", { name: "Compiled IR" }).click();
  await expect(page.locator("#ir-output")).not.toBeEmpty();
  await expect(page.locator("#instruction-count")).toContainText("Build #");
  await page.getByRole("button", { name: "Run program", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, world!\n");
  await expect(page.locator("#console-detail")).toContainText("Completed");

  await editProgram(page, (program) => {
    program.instructions.find(
      (instruction) => instruction.id === "audience",
    ).value = "Rivet!";
  });
  await expect(page.locator("#source-badge")).toHaveText("Modified");
  await expect(
    page.getByRole("button", { name: "Run program", exact: true }),
  ).toBeDisabled();
  await expect(page.locator("#build-status")).toContainText("build again");
  await build(page);
  await page.getByRole("button", { name: "Run program", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, Rivet!\n");
});

test("syntax, reference, and runtime errors appear as useful diagnostics", async ({
  page,
}) => {
  await page
    .getByRole("textbox", { name: "Program source" })
    .fill("{ invalid json");
  await page.locator("#build").click();
  await expect(page.getByRole("tab", { name: /Diagnostics/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await expect(page.locator("#diagnostics")).toContainText(/line|column/i);
  await expect(
    page.getByRole("button", { name: "Run program", exact: true }),
  ).toBeDisabled();

  await page.getByRole("button", { name: "Hello world", exact: true }).click();
  await editProgram(page, (program) => {
    program.entry.value = "missing_register";
  });
  await page.locator("#build").click();
  await expect(page.locator("#diagnostics")).toContainText("missing_register");
  await expect(page.locator("#diagnostics")).toContainText("entry");

  // This program is well typed; overflow is detected during execution.
  await page.getByRole("textbox", { name: "Program source" }).fill(`{
    "version": 1,
    "name": "Overflow",
    "instructions": [
      {"id":"max","op":"int","value":9223372036854775807},
      {"id":"one","op":"int","value":1},
      {"id":"sum","op":"add","left":"max","right":"one"},
      {"id":"text","op":"to_text","value":"sum"}
    ],
    "entry":{"kind":"console","value":"text"}
  }`);
  await build(page);
  await page.getByRole("button", { name: "Run program", exact: true }).click();
  await expect(page.locator("#diagnostics")).toContainText(/overflow/i);
});

test("HTTP service handles repeated real requests, missing routes, stop and restart", async ({
  page,
  request: api,
}) => {
  await startServer(page);
  const address = await page.locator("#server-address").textContent();
  expect(address).toMatch(/^http:\/\/127\.0\.0\.1:\d+$/);
  for (let count = 1; count <= 3; count += 1) {
    await request(page);
    await expect(page.locator("#response-meta")).toContainText("200");
    await expect(page.locator("#response-meta")).toContainText(
      "application/json",
    );
    await expect(page.getByLabel("HTTP response body")).toHaveText(
      JSON.stringify({ message: "hello world" }),
    );
    await expect(page.locator("#request-count")).toHaveText(String(count));
  }
  await request(page, "/missing");
  await expect(page.locator("#response-meta")).toContainText("404");
  await expect(page.locator("#request-count")).toHaveText("4");
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Stopped");
  await expect(
    page.getByRole("button", { name: "Send request" }),
  ).toBeDisabled();
  await expect(
    api.get(`${address}/hello`, { timeout: 1500 }),
  ).rejects.toThrow();
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page);
  await expect(page.locator("#response-meta")).toContainText("200");
  await expect(page.locator("#request-count")).toHaveText("1");
});

test("running artifact survives a failed build and edited server response is used after restart", async ({
  page,
}) => {
  await startServer(page);
  const original = await page
    .getByRole("textbox", { name: "Program source" })
    .inputValue();
  const runningBuild = await page.locator("#server-build").textContent();
  await page.getByRole("textbox", { name: "Program source" }).fill("{broken");
  await page.locator("#build").click();
  await expect(page.locator("#diagnostics")).not.toBeEmpty();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await expect(page.locator("#server-build")).toHaveText(runningBuild);
  await expect(
    page.getByRole("button", { name: "Stop", exact: true }),
  ).toBeEnabled();
  await request(page);
  await expect(page.getByLabel("HTTP response body")).toContainText(
    "hello world",
  );
  await expect(page.locator("#diagnostics")).not.toBeEmpty();

  const changed = JSON.parse(original);
  changed.instructions.find(
    (instruction) => instruction.id === "message",
  ).value = "from the edited program";
  changed.entry.routes[0].path = "/edited";
  await page
    .getByRole("textbox", { name: "Program source" })
    .fill(JSON.stringify(changed, null, 2));
  await build(page);
  await expect(
    page.getByRole("button", { name: "Start server", exact: true }),
  ).toBeDisabled();
  await expect(page.locator("#server-description")).toContainText(
    "still running",
  );
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Start server", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Request path" })).toHaveValue(
    "/edited",
  );
  await request(page, "/edited");
  await expect(page.getByLabel("HTTP response body")).toHaveText(
    JSON.stringify({ message: "from the edited program" }),
  );
});

test("computed example executes addition, conversion, and nested object composition", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Computed values", exact: true })
    .click();
  await build(page);
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page, "/computed");
  const body = JSON.parse(
    await page.getByLabel("HTTP response body").textContent(),
  );
  expect(body).toEqual({
    message: "The answer is 42",
    details: { answer: 42, ready: true },
  });
});

test("response inspector preserves signed 64-bit integers exactly", async ({
  page,
}) => {
  await page.getByRole("textbox", { name: "Program source" }).fill(`{
    "version": 1,
    "name": "Exact integer response",
    "instructions": [
      {"id":"max","op":"int","value":9223372036854775807},
      {"id":"response","op":"object","fields":{"max":"max"}}
    ],
    "entry":{"kind":"http","port":0,"routes":[{"method":"GET","path":"/max","status":200,"body":"response"}]}
  }`);
  await build(page);
  await page.getByRole("button", { name: "Start server", exact: true }).click();
  await expect(page.locator("#server-badge")).toHaveText("Running");
  await request(page, "/max");
  await expect(page.getByLabel("HTTP response body")).toHaveText(
    '{"max":9223372036854775807}',
  );
});

test("new, open, and download preserve editable program source", async ({
  page,
}) => {
  await page.getByRole("button", { name: "New program" }).click();
  await expect(page.locator("#program-name")).toHaveText("Untitled program");
  const document = {
    version: 1,
    name: "File roundtrip",
    instructions: [{ id: "message", op: "text", value: "Opened from disk" }],
    entry: { kind: "console", value: "message" },
  };
  const fileText = JSON.stringify(document, null, 2) + "\n";
  const fileChooserPromise = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Open file" }).click();
  await (
    await fileChooserPromise
  ).setFiles({
    name: "roundtrip.rivet.json",
    mimeType: "application/json",
    buffer: Buffer.from(fileText),
  });
  await expect(
    page.getByRole("textbox", { name: "Program source" }),
  ).toHaveValue(fileText);
  await build(page);
  await page.getByRole("button", { name: "Run program", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText(
    "Opened from disk\n",
  );
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download source" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("file-roundtrip.rivet.json");
  const chunks = [];
  for await (const chunk of await download.createReadStream())
    chunks.push(chunk);
  expect(Buffer.concat(chunks).toString()).toBe(fileText);
});

test("keyboard tabs, compact layout, and loading have no browser errors", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.getByRole("textbox", { name: "Program source" }).focus();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("tab", { name: "Console", exact: true }),
  ).toBeFocused();
  await page.getByRole("tab", { name: "Console", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("tab", { name: /Diagnostics/ })).toBeFocused();
  await expect(page.getByRole("tab", { name: /Diagnostics/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await page.keyboard.press("End");
  await expect(page.getByRole("tab", { name: "Compiled IR" })).toBeFocused();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(
    page.getByRole("textbox", { name: "Program source" }),
  ).toBeVisible();
  await expect(page.locator("#build")).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBeTruthy();
  await build(page);
  await page.getByRole("button", { name: "Run program", exact: true }).click();
  await expect(page.getByLabel("Program output")).toHaveText("Hello, world!\n");
  expect(errors).toEqual([]);
});
