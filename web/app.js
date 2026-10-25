(() => {
  "use strict";

  const $ = (id) => document.getElementById(id);
  const source = $("source");
  const encoder = new TextEncoder();
  const state = {
    examples: [],
    selectedExample: null,
    build: null,
    builtSource: null,
    server: null,
    busy: null,
    connected: false,
    loaded: false,
    diagnostics: [],
    diagnosticAction: null,
    epoch: 0,
    statusPending: false,
    runBuildId: null,
  };

  async function api(path, body) {
    const options =
      body === undefined
        ? {}
        : {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-Rivet-Request": "1",
            },
            body: JSON.stringify(body),
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

  function programInfo() {
    try {
      const value = JSON.parse(source.value);
      return {
        name:
          typeof value?.name === "string" && value.name.trim()
            ? value.name
            : "Untitled program",
        kind: value?.entry?.kind,
      };
    } catch {
      return { name: "Untitled program", kind: null };
    }
  }

  function isCurrentBuild() {
    return Boolean(state.build && state.builtSource === source.value);
  }

  function renderEditor() {
    const lines = source.value.split("\n").length;
    $("line-numbers").textContent = Array.from(
      { length: lines },
      (_, index) => index + 1,
    ).join("\n");
    $("line-numbers").scrollTop = source.scrollTop;
    const bytes = encoder.encode(source.value).length;
    const size =
      bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`;
    $("source-stats").textContent =
      `${lines} ${lines === 1 ? "line" : "lines"} · ${size}`;
    $("program-name").textContent = programInfo().name;
    for (const button of $("examples").querySelectorAll("button")) {
      button.setAttribute(
        "aria-current",
        String(button.dataset.id === state.selectedExample),
      );
    }
    renderControls();
  }

  function renderControls() {
    const current = isCurrentBuild();
    const busy = Boolean(state.busy);
    const kind = programInfo().kind;
    $("build").disabled = busy || !state.loaded;
    $("run").disabled = busy || !current || Boolean(state.server);
    $("stop").disabled = busy || !state.server;
    $("send-request").disabled = busy || !state.server;
    $("request-path").disabled = busy;
    $("run-label").textContent =
      state.busy === "run"
        ? "Executing…"
        : kind === "http"
          ? "Start server"
          : "Run program";
    $("build").querySelector("span").textContent =
      state.busy === "build" ? "Building…" : "Build";
    $("source-badge").textContent = current
      ? "Built"
      : state.build
        ? "Modified"
        : "Unbuilt";
    $("source-badge").className =
      `badge ${current ? "success" : state.build ? "warning" : "neutral"}`;
    let status = "Build to validate this program";
    let tone = "";
    if (state.busy) {
      status = {
        build: "Compiling and checking types…",
        run: "Executing compiled program…",
        stop: "Stopping and releasing the listener…",
        request: "Sending a local HTTP request…",
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
    $("ir-empty").hidden = Boolean(state.build);
    $("ir-output").hidden = !state.build;
    $("ir-output").textContent = state.build?.ir || "";
    $("instruction-count").textContent = state.build
      ? `Build #${state.build.id}${current ? "" : " · previous source"}`
      : "";
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
        ? "Your compiled program is accepting local requests."
        : `Build #${server.build_id} is still running. Stop it to run a new build.`;
    } else {
      $("server-address").removeAttribute("href");
      $("server-description").textContent =
        "Build a program with an HTTP entry, then start your local service.";
    }
    $("request-count").textContent = server ? String(server.requests) : "—";
    $("server-build").textContent = server ? `#${server.build_id}` : "—";
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

  function setSource(value, selectedExample = null) {
    source.value = value;
    source.scrollTop = 0;
    source.scrollLeft = 0;
    state.selectedExample = selectedExample;
    showNotice("");
    setDiagnostics([]);
    renderEditor();
    renderServer();
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
      if (!error.status) setConnected(false);
      const diagnostics = error.diagnostics?.length
        ? error.diagnostics
        : [{ path: action, message: error.message }];
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
      if (state.build?.id !== result.build?.id) state.builtSource = null;
      state.build = result.build;
      state.server = result.server;
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
      const submittedSource = source.value;
      state.build = null;
      state.builtSource = null;
      setDiagnostics([]);
      const result = await api("build", { source: submittedSource });
      state.build = result.build;
      state.builtSource = submittedSource;
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
      const result = await api("run", { build_id: build.id });
      setDiagnostics([]);
      state.runBuildId = build.id;
      $("console-empty").hidden = true;
      $("console-detail").hidden = false;
      if (result.kind === "console") {
        $("console-output").hidden = false;
        $("console-output").textContent = result.stdout;
        $("run-timing").textContent = duration(result.elapsed_us);
        $("console-detail").textContent =
          `Build #${build.id} · ${build.name} · Completed`;
      } else {
        state.server = result.server;
        $("console-output").hidden = true;
        $("console-output").textContent = "";
        $("run-timing").textContent = "Listening";
        $("console-detail").textContent =
          `Build #${build.id} · ${build.name}\nHTTP service started at ${result.server.address}.\nUse the request inspector below to send a real request.`;
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
          `Build #${oldServer.build_id} · HTTP service stopped. Listener released.`;
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
    state.selectedExample =
      state.examples.find((example) => example.source === source.value)?.id ||
      null;
    renderEditor();
    renderServer();
  });
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

  $("new-program").addEventListener("click", () => {
    setSource(
      JSON.stringify(
        {
          version: 1,
          name: "Untitled program",
          instructions: [{ id: "message", op: "text", value: "" }],
          entry: { kind: "console", value: "message" },
        },
        null,
        2,
      ) + "\n",
    );
    source.focus();
  });

  $("open-program").addEventListener("click", () => $("file-input").click());
  $("file-input").addEventListener("change", async () => {
    const file = $("file-input").files[0];
    if (!file) return;
    try {
      if (file.size > 64 * 1024)
        throw new Error("This file exceeds the 64 KiB program source limit.");
      setSource(await file.text());
      showNotice(`Opened ${file.name}. Build to validate and run it.`);
    } catch (error) {
      showNotice(error.message, true);
    } finally {
      $("file-input").value = "";
    }
  });

  $("download-program").addEventListener("click", () => {
    const name =
      programInfo()
        .name.toLowerCase()
        .replace(/[^a-z0-9_-]+/g, "-")
        .replace(/^-+|-+$/g, "")
        .slice(0, 60) || "program";
    const url = URL.createObjectURL(
      new Blob([source.value], { type: "application/json" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = `${name}.rivet.json`;
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
        button.addEventListener("click", () =>
          setSource(example.source, example.id),
        );
        $("examples").append(button);
      }
      state.loaded = true;
      if (state.examples[0])
        setSource(state.examples[0].source, state.examples[0].id);
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
