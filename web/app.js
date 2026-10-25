(() => {
  "use strict";

  const $ = (id) => document.getElementById(id);
  const source = $("source");
  const encoder = new TextEncoder();
  const state = {
    examples: [],
    selectedExample: null,
    build: null,
    builtBytes: null,
    bytes: null,
    filename: "untitled.rvb",
    hexText: "",
    inspection: null,
    inspectionPending: false,
    revision: 0,
    literalSelection: "0",
    inspectTimer: null,
    server: null,
    busy: null,
    connected: false,
    loaded: false,
    diagnostics: [],
    diagnosticAction: null,
    epoch: 0,
    statusPending: false,
    runBuildId: null,
    lastRuntimeError: null,
    runInputSnapshot: null,
  };

  async function api(path, body) {
    const options =
      body === undefined
        ? {}
        : {
            method: "POST",
            headers: {
              "Content-Type": body instanceof Uint8Array ? "application/octet-stream" : "application/json",
              "X-Rivet-Request": "1",
            },
            body: body instanceof Uint8Array ? body : JSON.stringify(body),
          };
    const response = await fetch(`/api/${path}`, {
      ...options,
      cache: "no-store",
    });
    let result;
    try {
      result = await response.json();
    } catch {
      throw new Error(
        `The workbench returned an unreadable response (${response.status}).`,
      );
    }
    setConnected(true);
    if (!response.ok || result.ok === false) {
      const error = new Error(
        result.error || `Request failed (${response.status}).`,
      );
      error.diagnostics = result.diagnostics || [];
      error.status = response.status;
      error.exitCode = result.exit_code;
      error.stdout = result.stdout;
      error.stderr = result.stderr;
      throw error;
    }
    return result;
  }

  function setConnected(connected) {
    state.connected = connected;
    $("connection").className =
      `connection-pill ${connected ? "connected" : "disconnected"}`;
    $("connection-label").textContent = connected
      ? `Local · ${location.port || "80"}`
      : "Disconnected";
  }

  function showNotice(message, isError = false) {
    $("notice").textContent = message;
    $("notice").className = `notice${isError ? " error" : ""}`;
    $("notice").hidden = !message;
  }

  function exitStatus(code) {
    return Number.isInteger(code) ? `Exit code ${code}` : "No exit code reported";
  }

  function programInfo() {
    return { name: state.filename, kind: state.inspection?.entry?.kind };
  }

  function sourceFormat() {
    const bytes = state.bytes;
    if (!bytes || bytes.length < 6 || bytes[0] !== 0x52 || bytes[1] !== 0x56 || bytes[2] !== 0x42 || bytes[3] !== 0)
      return "Binary";
    return `RVB${new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint16(4, true)}`;
  }

  function equalBytes(left, right) {
    return left && right && left.length === right.length &&
      left.every((value, index) => value === right[index]);
  }

  function isCurrentBuild() {
    return Boolean(state.build && equalBytes(state.builtBytes, state.bytes));
  }

  function formatHex(bytes) {
    return Array.from({ length: Math.ceil(bytes.length / 16) }, (_, line) =>
      Array.from(bytes.subarray(line * 16, line * 16 + 16),
        value => value.toString(16).padStart(2, "0")).join(" ")).join("\n");
  }

  function parseHex(text) {
    const compact = text.replace(/\s/g, "");
    if (!/^[0-9a-f]*$/i.test(compact))
      throw localError("Use hexadecimal byte pairs (00–ff), separated by spaces or newlines.");
    if (compact.length % 2) throw localError("A byte needs two hexadecimal digits. Complete the final pair.");
    if (compact.length > 65536 * 2) throw localError("Binary source exceeds the 64 KiB limit.");
    return Uint8Array.from(compact.match(/../g) || [], pair => parseInt(pair, 16));
  }

  function localError(message) {
    const error = new Error(message);
    error.local = true;
    return error;
  }

  function renderSource() {
    const hex = $("source-view").value === "hex";
    source.value = hex ? state.hexText : state.inspection?.ir ||
      (state.inspectionPending ? "Decoding binary source…" : "No decoded view. Inspect Diagnostics or repair the Hex bytes.");
    source.setAttribute("aria-label", hex ? "Binary source bytes" : "Decoded program");
    $("source-help").textContent = hex
      ? "Hex editor · Each pair represents one source byte"
      : `Decoded from ${sourceFormat()} bytes · Read-only view`;
    renderEditor();
  }

  function renderEditor() {
    const hex = $("source-view").value === "hex";
    let precedingDigits = 0;
    $("line-numbers").textContent = source.value.split("\n").map((line, index) => {
      const label = hex ? Math.floor(precedingDigits / 2).toString(16).padStart(4, "0") : index + 1;
      precedingDigits += line.replace(/\s/g, "").length;
      return label;
    }).join("\n");
    $("line-numbers").scrollTop = source.scrollTop;
    $("source-stats").textContent = state.bytes
      ? `${state.bytes.length.toLocaleString()} bytes · ${sourceFormat()}`
      : "Incomplete hex bytes";
    $("program-name").textContent = state.filename;
    for (const button of $("examples").querySelectorAll("button"))
      button.setAttribute("aria-current", String(button.dataset.id === state.selectedExample));
    renderControls();
  }

  function renderLiteralSelection() {
    const instruction = state.inspection?.instructions.find(item => String(item.index) === $("literal-select").value);
    $("literal-value").value = instruction?.literal ?? "";
    $("literal-label").textContent = instruction ? `${instruction.type_name} literal` : "Literal value";
    $("literal-location").textContent = instruction
      ? `r${instruction.index} · payload 0x${instruction.payload_offset.toString(16).padStart(4, "0")} · ${instruction.payload_length} bytes`
      : "";
  }

  function renderValueEditors() {
    const inspection = state.inspection;
    const selected = state.literalSelection;
    $("literal-select").replaceChildren();
    const literals = inspection?.instructions.filter(item => typeof item.literal === "string") || [];
    $("binary-editors").hidden = !literals.length && inspection?.entry.kind !== "http";
    for (const instruction of literals) {
      const option = document.createElement("option");
      option.value = String(instruction.index);
      option.textContent = `r${instruction.index} · ${instruction.type_name} · 0x${instruction.offset.toString(16).padStart(4, "0")}`;
      $("literal-select").append(option);
    }
    if (literals.some(item => String(item.index) === selected)) $("literal-select").value = selected;
    $("literal-editor").hidden = !literals.length;
    renderLiteralSelection();
    $("port-editor").hidden = inspection?.entry.kind !== "http";
    if (inspection?.entry.kind === "http")
      $("source-port").value = new DataView(state.bytes.buffer, state.bytes.byteOffset, state.bytes.byteLength).getUint16(inspection.entry_offset, true);
  }

  async function inspectSource(bytes, revision) {
    try {
      const result = await api("inspect", bytes);
      if (revision !== state.revision) return;
      state.inspection = result.inspection;
      if (state.diagnosticAction === "source") setDiagnostics([]);
    } catch (error) {
      if (revision !== state.revision) return;
      state.inspection = null;
      if (!error.status) setConnected(false);
      setDiagnostics(error.diagnostics?.length ? error.diagnostics :
        [{ path: "binary source", message: error.message }], "source");
      selectTab("diagnostics");
    } finally {
      if (revision === state.revision) {
        state.inspectionPending = false;
        renderValueEditors();
        if ($("source-view").value === "decoded") renderSource();
        else renderEditor();
        renderServer();
      }
    }
  }

  async function setSourceBytes(bytes, filename = state.filename, selectedExample = null, input) {
    clearTimeout(state.inspectTimer);
    state.bytes = bytes.slice();
    state.filename = filename;
    state.hexText = formatHex(bytes);
    state.selectedExample = selectedExample;
    if (input !== undefined) {
      $("runtime-input").value = input;
      state.runInputSnapshot = null;
    }
    state.inspection = null;
    state.inspectionPending = true;
    const revision = ++state.revision;
    state.epoch += 1;
    showNotice("");
    setDiagnostics([]);
    source.scrollTop = 0;
    source.scrollLeft = 0;
    renderValueEditors();
    renderSource();
    renderServer();
    await inspectSource(state.bytes, revision);
  }

  async function loadExample(example) {
    await perform("open", async () => {
      const response = await fetch(example.url, { cache: "no-store" });
      if (!response.ok) throw new Error(`Cannot load binary example (${response.status}).`);
      const bytes = new Uint8Array(await response.arrayBuffer());
      await setSourceBytes(bytes, example.filename, example.id, example.input || "");
    });
  }

  function renderControls() {
    const current = isCurrentBuild();
    const busy = Boolean(state.busy);
    const kind = programInfo().kind;
    const inputBytes = encoder.encode($("runtime-input").value).length;
    const inputTooLarge = kind === "compute" && inputBytes > 262144;
    $("runtime-input-panel").hidden = kind !== "compute";
    $("runtime-input").disabled = busy;
    $("runtime-input").setAttribute("aria-invalid", String(inputTooLarge));
    const exampleHelp = {
      "two-sum": "two-sum-input-help",
      "add-two-numbers": "add-two-numbers-input-help",
    }[state.selectedExample];
    for (const id of ["two-sum-input-help", "add-two-numbers-input-help"])
      $(id).hidden = id !== exampleHelp;
    $("runtime-input").setAttribute("aria-describedby",
      `runtime-input-help${exampleHelp ? ` ${exampleHelp}` : ""}`);
    $("runtime-input-size").textContent = `${inputBytes.toLocaleString()} bytes · 256 KiB maximum`;
    $("runtime-input-size").classList.toggle("too-large", inputTooLarge);
    $("runtime-input-status").textContent = inputTooLarge
      ? "Input is too large · shorten it before running"
      : state.runInputSnapshot !== null && state.runBuildId === state.build?.id && $("runtime-input").value !== state.runInputSnapshot
        ? "Input changed · run the same executable again"
        : "Passed to each run · no rebuild needed";
    $("build").disabled = busy || !state.loaded || !state.bytes || state.inspectionPending;
    source.readOnly = busy || $("source-view").value !== "hex";
    $("download-program").disabled = busy || !state.bytes;
    $("new-program").disabled = busy;
    $("open-program").disabled = busy;
    for (const button of $("examples").querySelectorAll("button")) button.disabled = busy;
    for (const id of ["literal-select", "literal-value", "apply-literal", "source-port", "apply-port"])
      $(id).disabled = busy || state.inspectionPending || !state.inspection;
    $("inspection-status").textContent = state.inspectionPending ? "Decoding…" : state.inspection
      ? `${state.inspection.instructions.length} instructions · Valid binary`
      : state.bytes ? "Invalid binary" : "Incomplete bytes";
    $("run").disabled = busy || !current || state.inspectionPending || !state.inspection || Boolean(state.server) || inputTooLarge;
    $("stop").disabled = busy || !state.server;
    $("send-request").disabled = busy || !state.server;
    $("request-path").disabled = busy;
    $("run-label").textContent =
      state.busy === "run"
        ? "Executing…"
        : kind === "http"
          ? "Start server"
          : "Run executable";
    $("build").querySelector("span").textContent =
      state.busy === "build" ? "Building…" : "Build";
    $("source-badge").textContent = current
      ? "Built"
      : state.build
        ? "Modified"
        : "Unbuilt";
    $("source-badge").className =
      `badge ${current ? "success" : state.build ? "warning" : "neutral"}`;
    let status = "Build to create a native executable";
    let tone = "";
    if (state.busy) {
      status = {
        build: "Checking types and compiling a native executable…",
        run: "Launching the native executable…",
        stop: "Stopping the native process and releasing its listener…",
        request: "Sending a local HTTP request…",
        open: "Opening binary source…",
        edit: "Updating and decoding binary source…",
      }[state.busy];
    } else if (state.diagnostics.length) {
      status = `${state.diagnostics.length} ${state.diagnostics.length === 1 ? "diagnostic" : "diagnostics"} · inspect the Diagnostics tab`;
      tone = "error";
    } else if (current) {
      status = `Build #${state.build.id} ready · ${state.build.instructions} ${state.build.instructions === 1 ? "instruction" : "instructions"}`;
      tone = "success";
    } else if (state.build) {
      status = "Source changed · build again before running";
      tone = "warning";
    }
    $("build-status").textContent = status;
    $("build-dot").className = `status-dot ${tone}`;
    renderCode();
    renderArtifact();
  }

  function renderCode() {
    const view = $("code-view").value;
    const labels = {
      ir: "Compiled intermediate representation",
      llvm_ir: "LLVM intermediate representation",
      assembly: "Generated assembly",
    };
    $("code-view").disabled = !state.build;
    $("ir-empty").hidden = Boolean(state.build);
    $("ir-output").hidden = !state.build;
    $("ir-output").textContent = state.build?.[view] || "";
    $("ir-output").setAttribute("aria-label", labels[view]);
    $("instruction-count").textContent = state.build
      ? `Build #${state.build.id}${isCurrentBuild() ? "" : " · previous source"}`
      : "";
  }

  function renderArtifact() {
    const artifact = state.build?.artifact;
    $("artifact-details").hidden = !artifact;
    $("download-executable").disabled = !artifact || Boolean(state.busy);
    $("artifact-state").textContent = artifact
      ? `Build #${state.build.id} · ${isCurrentBuild() ? "Current source" : "Previous source — rebuild to include edits"}`
      : "Build to create a standalone executable.";
    $("artifact-path").textContent = artifact?.path || "";
    $("artifact-size").textContent = artifact
      ? `${artifact.size_bytes.toLocaleString()} bytes`
      : "—";
    $("artifact-profile").textContent = artifact?.profile || "—";
    $("artifact-target").textContent = artifact?.target || "—";
    $("artifact-build-time").textContent = artifact
      ? `${state.build.build_ms.toLocaleString()} ms`
      : "—";
  }

  function renderServer() {
    const server = state.server;
    $("server-badge").textContent = server ? "Running" : "Stopped";
    $("server-badge").className = `badge ${server ? "success" : "neutral"}`;
    $("server-placeholder").hidden = Boolean(server);
    $("server-address").hidden = !server;
    if (server) {
      $("server-address").textContent = server.address;
      $("server-address").href = server.address + (server.routes[0] || "/");
      const latest = state.build?.id === server.build_id && isCurrentBuild();
      $("server-description").textContent = latest
        ? "Your native executable is accepting local requests."
        : `Build #${server.build_id} is still running. Stop it to run a new build.`;
    } else {
      $("server-address").removeAttribute("href");
      $("server-description").textContent =
        "Build a program with an HTTP entry, then start your local service.";
    }
    $("request-count").textContent = server ? String(server.requests) : "—";
    $("server-build").textContent = server ? `#${server.build_id}` : "—";
    $("server-pid").textContent = server ? String(server.pid) : "—";
    $("route-list").replaceChildren();
    for (const path of server?.routes || []) {
      const chip = document.createElement("span");
      chip.className = "route-chip";
      chip.textContent = `GET ${path}`;
      $("route-list").append(chip);
    }
    renderControls();
  }

  function setDiagnostics(diagnostics, action = null) {
    state.diagnostics = diagnostics;
    state.diagnosticAction = action;
    $("diagnostic-count").textContent = String(diagnostics.length);
    $("diagnostic-count").className =
      `tab-count${diagnostics.length ? " has-errors" : ""}`;
    $("diagnostics-empty").hidden = diagnostics.length > 0;
    $("diagnostics").replaceChildren();
    for (const diagnostic of diagnostics) {
      const item = document.createElement("li");
      const path = document.createElement("code");
      path.textContent = diagnostic.path || "program";
      const message = document.createElement("p");
      message.textContent = diagnostic.message;
      item.append(path, message);
      $("diagnostics").append(item);
    }
    renderControls();
  }

  function selectTab(name, focus = false) {
    for (const tab of document.querySelectorAll('[role="tab"]')) {
      const selected = tab.id === `tab-${name}`;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
      $(tab.getAttribute("aria-controls")).hidden = !selected;
      if (selected && focus) tab.focus();
    }
  }

  async function perform(action, work) {
    if (state.busy) return;
    state.busy = action;
    state.epoch += 1;
    showNotice("");
    renderControls();
    try {
      await work();
    } catch (error) {
      if (!error.status && !error.local) setConnected(false);
      const diagnostics = error.diagnostics?.length
        ? error.diagnostics
        : [{ path: action, message: error.message }];
      if (action === "run" && error.exitCode !== undefined) {
        $("console-empty").hidden = true;
        $("console-output").hidden = !error.stdout;
        $("console-output").textContent = error.stdout || "";
        $("console-detail").hidden = false;
        $("run-timing").textContent = "Exited";
        $("console-detail").textContent =
          `Build #${state.build.id} · ${exitStatus(error.exitCode)}${error.stderr ? `\n${error.stderr}` : ""}`;
      }
      setDiagnostics(diagnostics, action);
      selectTab("diagnostics");
      showNotice(error.message, true);
    } finally {
      state.busy = null;
      renderControls();
      await refreshStatus();
    }
  }

  async function refreshStatus() {
    if (state.busy || state.statusPending) return;
    const epoch = state.epoch;
    state.statusPending = true;
    try {
      const result = await api("status");
      if (epoch !== state.epoch || state.busy) return;
      if (
        state.server &&
        !result.server &&
        state.server.build_id === state.runBuildId
      ) {
        $("run-timing").textContent = "Exited";
        $("console-detail").textContent =
          `Build #${state.server.build_id} · Native process ${state.server.pid} exited. Listener is no longer running.`;
      }
      if (state.build?.id !== result.build?.id) state.builtBytes = null;
      state.build = result.build;
      state.server = result.server;
      const runtimeError = result.last_error || null;
      if (runtimeError && runtimeError !== state.lastRuntimeError) {
        const hadDiagnostics = state.diagnostics.length > 0;
        setDiagnostics(
          [...state.diagnostics, { path: "runtime", message: runtimeError }],
          state.diagnosticAction || "run",
        );
        if (!hadDiagnostics) showNotice(runtimeError, true);
      }
      state.lastRuntimeError = runtimeError;
      renderServer();
    } catch {
      if (epoch === state.epoch) setConnected(false);
    } finally {
      state.statusPending = false;
    }
  }

  function duration(microseconds) {
    return microseconds < 1000
      ? `${microseconds} µs`
      : `${(microseconds / 1000).toFixed(2)} ms`;
  }

  function resetResponse() {
    $("response-meta").replaceChildren();
    $("response-body").textContent =
      "Send a request to inspect the real HTTP response.";
    $("response-body").className = "response-body";
  }

  $("build").addEventListener("click", () =>
    perform("build", async () => {
      const submittedSource = state.bytes.slice();
      state.build = null;
      state.builtBytes = null;
      setDiagnostics([]);
      const result = await api("build", submittedSource);
      state.build = result.build;
      state.builtBytes = submittedSource;
      renderEditor();
      renderServer();
      if (
        document.querySelector('[role="tab"][aria-selected="true"]').id ===
        "tab-diagnostics"
      )
        selectTab("ir");
    }),
  );

  $("run").addEventListener("click", () =>
    perform("run", async () => {
      if (!isCurrentBuild())
        throw new Error("Build the current source before running.");
      const build = state.build;
      const request = { build_id: build.id };
      if (programInfo().kind === "compute") {
        request.input = $("runtime-input").value;
        state.runInputSnapshot = request.input;
      }
      const result = await api("run", request);
      setDiagnostics([]);
      state.runBuildId = build.id;
      $("console-empty").hidden = true;
      $("console-detail").hidden = false;
      if (result.kind === "console") {
        $("console-output").hidden = false;
        $("console-output").textContent = result.stdout;
        $("run-timing").textContent = duration(result.elapsed_us);
        $("console-detail").textContent =
          `Build #${build.id} · ${build.name} · Completed · ${exitStatus(result.exit_code)}`;
      } else {
        state.server = result.server;
        $("console-output").hidden = true;
        $("console-output").textContent = "";
        $("run-timing").textContent = "Listening";
        $("console-detail").textContent =
          `Build #${build.id} · ${build.name} · Process ${result.server.pid}\nNative HTTP service started at ${result.server.address}.\nUse the request inspector below to send a real request.`;
        $("request-path").value = result.server.routes[0] || "/";
        resetResponse();
        renderServer();
      }
      selectTab("console");
    }),
  );

  $("stop").addEventListener("click", () =>
    perform("stop", async () => {
      const oldServer = state.server;
      await api("stop", {});
      state.server = null;
      if (oldServer?.build_id === state.runBuildId) {
        $("run-timing").textContent = "Stopped";
        $("console-detail").textContent =
          `Build #${oldServer.build_id} · Native process ${oldServer.pid} stopped. Listener released.`;
      }
      renderServer();
    }),
  );

  $("request-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!state.server || state.busy) return;
    perform("request", async () => {
      const result = await api("request", { path: $("request-path").value });
      // Keep the actual wire body: JSON.parse would round valid Rivet Int values
      // beyond JavaScript's exact-integer range before displaying them.
      $("response-body").textContent = result.body;
      $("response-body").className = "response-body has-response";
      const status = document.createElement("span");
      status.className = `response-status${result.status >= 400 ? " unsuccessful" : ""}`;
      status.textContent = String(result.status);
      const contentType = document.createElement("span");
      contentType.textContent = result.content_type;
      const timing = document.createElement("span");
      timing.textContent = duration(result.elapsed_us);
      $("response-meta").replaceChildren(status, contentType, timing);
      if (state.diagnosticAction === "request") setDiagnostics([]);
    });
  });

  source.addEventListener("input", () => {
    if ($("source-view").value !== "hex") return;
    clearTimeout(state.inspectTimer);
    state.hexText = source.value;
    state.inspection = null;
    state.selectedExample = null;
    const revision = ++state.revision;
    state.epoch += 1;
    showNotice("");
    try {
      state.bytes = parseHex(source.value);
      state.inspectionPending = true;
      setDiagnostics([]);
      const bytes = state.bytes;
      state.inspectTimer = setTimeout(() => inspectSource(bytes, revision), 150);
    } catch (error) {
      state.bytes = null;
      state.inspectionPending = false;
      setDiagnostics([{ path: "hex bytes", message: error.message }], "source");
      selectTab("diagnostics");
    }
    renderValueEditors();
    renderEditor();
    renderServer();
  });
  $("source-view").addEventListener("change", renderSource);
  $("runtime-input").addEventListener("input", renderControls);
  $("literal-select").addEventListener("change", () => {
    state.literalSelection = $("literal-select").value;
    renderLiteralSelection();
  });
  $("apply-literal").addEventListener("click", () => perform("edit", async () => {
    const instruction = state.inspection?.instructions.find(item => String(item.index) === $("literal-select").value);
    if (!instruction) throw localError("Select a literal instruction first.");
    const value = $("literal-value").value;
    let payload;
    if (instruction.opcode === 1) {
      payload = encoder.encode(value);
      if (payload.length > 16384) throw localError("Text literals are limited to 16 KiB of UTF-8 bytes.");
    } else if (instruction.opcode === 2) {
      if (!/^-?\d+$/.test(value)) throw localError("Enter a whole signed 64-bit integer.");
      const integer = BigInt(value);
      if (integer < -(1n << 63n) || integer > (1n << 63n) - 1n)
        throw localError("Integer is outside the signed 64-bit range.");
      payload = new Uint8Array(8);
      new DataView(payload.buffer).setBigInt64(0, integer, true);
    } else if (instruction.opcode === 3) {
      if (value !== "true" && value !== "false") throw localError("A boolean literal must be true or false.");
      payload = Uint8Array.of(value === "true" ? 1 : 0);
    } else throw localError("This instruction has no editable scalar literal.");
    const length = state.bytes.length - instruction.payload_length + payload.length;
    if (length > 65536) throw localError("The edited source would exceed 64 KiB.");
    const bytes = new Uint8Array(length);
    bytes.set(state.bytes.subarray(0, instruction.payload_offset));
    bytes.set(payload, instruction.payload_offset);
    bytes.set(state.bytes.subarray(instruction.payload_offset + instruction.payload_length), instruction.payload_offset + payload.length);
    const view = new DataView(bytes.buffer);
    view.setUint16(instruction.offset + 2, payload.length, true);
    view.setUint32(12, length - 16, true);
    await setSourceBytes(bytes);
  }));
  $("apply-port").addEventListener("click", () => perform("edit", async () => {
    const value = $("source-port").value;
    if (!/^\d+$/.test(value) || Number(value) > 65535)
      throw localError("Server port must be an integer from 0 to 65535.");
    if (state.inspection?.entry.kind !== "http") throw localError("This source has no HTTP entry.");
    const bytes = state.bytes.slice();
    new DataView(bytes.buffer).setUint16(state.inspection.entry_offset, Number(value), true);
    await setSourceBytes(bytes);
  }));
  source.addEventListener("scroll", () => {
    $("line-numbers").scrollTop = source.scrollTop;
  });
  document.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      if (!$("build").disabled) $("build").click();
    }
  });
  if (!/Mac|iPhone|iPad/.test(navigator.platform))
    $("build-shortcut").textContent = "Ctrl ↵";

  $("code-view").addEventListener("change", renderCode);

  $("download-executable").addEventListener("click", () => {
    const artifact = state.build?.artifact;
    if (!artifact) return;
    const link = document.createElement("a");
    link.href = artifact.download_url;
    link.download = "rivet-program";
    link.click();
  });

  for (const tab of document.querySelectorAll('[role="tab"]')) {
    tab.addEventListener("click", () => selectTab(tab.id.slice(4)));
    tab.addEventListener("keydown", (event) => {
      const tabs = [...document.querySelectorAll('[role="tab"]')];
      const index = tabs.indexOf(tab);
      let next;
      if (event.key === "ArrowRight") next = (index + 1) % tabs.length;
      if (event.key === "ArrowLeft")
        next = (index + tabs.length - 1) % tabs.length;
      if (event.key === "Home") next = 0;
      if (event.key === "End") next = tabs.length - 1;
      if (next !== undefined) {
        event.preventDefault();
        selectTab(tabs[next].id.slice(4), true);
      }
    });
  }

  $("new-program").addEventListener("click", () => perform("open", async () => {
    // RVB1 header, one empty Text record, console entry referencing register 0.
    const bytes = Uint8Array.of(0x52, 0x56, 0x42, 0, 1, 0, 0, 0, 1, 0, 1, 0, 6, 0, 0, 0, 1, 1, 0, 0, 0, 0);
    await setSourceBytes(bytes, "untitled.rvb", null, "");
  }));

  $("open-program").addEventListener("click", () => $("file-input").click());
  $("file-input").addEventListener("change", async () => {
    const file = $("file-input").files[0];
    if (!file) return;
    await perform("open", async () => {
      if (file.size > 65536) throw localError("This file exceeds the 64 KiB binary source limit.");
      await setSourceBytes(new Uint8Array(await file.arrayBuffer()), file.name, null, "");
      if (state.inspection) showNotice(`Opened ${file.name} as binary source. Build to create its executable.`);
    });
    $("file-input").value = "";
  });

  $("download-program").addEventListener("click", () => {
    if (!state.bytes) return;
    const filename = state.filename.replace(/\.[^.]+$/, "").replace(/[^a-z0-9_-]+/gi, "-").slice(0, 80) || "program";
    const url = URL.createObjectURL(new Blob([state.bytes], { type: "application/octet-stream" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = `${filename}.rvb`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  });

  async function initialize() {
    try {
      const result = await api("examples");
      state.examples = result.examples;
      $("examples").replaceChildren();
      for (const [index, example] of state.examples.entries()) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "example-button";
        button.dataset.id = example.id;
        const number = document.createElement("span");
        number.className = "example-number";
        number.textContent = String(index + 1).padStart(2, "0");
        number.setAttribute("aria-hidden", "true");
        const name = document.createElement("span");
        name.textContent = example.name;
        const arrow = document.createElement("span");
        arrow.className = "example-arrow";
        arrow.textContent = "↗";
        arrow.setAttribute("aria-hidden", "true");
        button.append(number, name, arrow);
        button.addEventListener("click", () => loadExample(example));
        $("examples").append(button);
      }
      state.loaded = true;
      if (state.examples[0]) await loadExample(state.examples[0]);
      await refreshStatus();
    } catch (error) {
      setConnected(false);
      showNotice(
        `Unable to load the workbench: ${error.message} Refresh this page after the local runtime is available.`,
        true,
      );
      $("examples").textContent = "Examples unavailable";
    }
  }

  initialize();
  setInterval(() => {
    if (!document.hidden) refreshStatus();
  }, 1000);
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) refreshStatus();
  });
})();
