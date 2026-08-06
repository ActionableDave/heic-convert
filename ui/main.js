const tauri = window.__TAURI__;

const dropzone = document.getElementById("dropzone");
const browseBtn = document.getElementById("browse");
const formatSel = document.getElementById("format");
const qualityRow = document.getElementById("quality-row");
const qualitySlider = document.getElementById("quality");
const qualityVal = document.getElementById("quality-val");
const outdirLabel = document.getElementById("outdir-label");
const fileListEl = document.getElementById("filelist");
const summaryEl = document.getElementById("summary");
const clearBtn = document.getElementById("clear");
const convertBtn = document.getElementById("convert");

/** @type {Map<string, {status: string, row: HTMLElement}>} path -> state */
const files = new Map();
let customOutDir = null;
let converting = false;

// ---------- adding files ----------

function isHeic(path) {
  return /\.(heic|heif)$/i.test(path);
}

function addFiles(paths) {
  if (converting) return;
  let added = 0;
  for (const p of paths) {
    if (!isHeic(p) || files.has(p)) continue;
    const row = document.createElement("div");
    row.className = "file-row";
    const name = document.createElement("span");
    name.className = "file-name";
    name.textContent = p.split(/[\\/]/).pop();
    name.title = p;
    const status = document.createElement("span");
    status.className = "file-status";
    status.textContent = "queued";
    const remove = document.createElement("button");
    remove.className = "file-remove";
    remove.textContent = "×";
    remove.title = "Remove";
    remove.addEventListener("click", () => {
      if (converting) return;
      files.delete(p);
      row.remove();
      refreshUi();
    });
    row.append(name, status, remove);
    fileListEl.appendChild(row);
    files.set(p, { status: "queued", row });
    added++;
  }
  if (added === 0 && paths.length > 0 && files.size === 0) {
    summaryEl.textContent = "No HEIC files in that selection";
  }
  refreshUi();
}

function refreshUi(keepSummary = false) {
  fileListEl.hidden = files.size === 0;
  clearBtn.hidden = files.size === 0;
  convertBtn.disabled = converting || files.size === 0;
  clearBtn.disabled = converting;
  if (keepSummary) return;
  if (files.size > 0 && !converting) {
    summaryEl.textContent = `${files.size} file${files.size === 1 ? "" : "s"} ready`;
  } else if (files.size === 0) {
    summaryEl.textContent = "";
  }
}

// ---------- drag & drop ----------

tauri.event.listen("tauri://drag-enter", () => dropzone.classList.add("hover"));
tauri.event.listen("tauri://drag-leave", () => dropzone.classList.remove("hover"));
tauri.event.listen("tauri://drag-drop", (e) => {
  dropzone.classList.remove("hover");
  addFiles(e.payload.paths || []);
});

// ---------- pickers ----------

browseBtn.addEventListener("click", async () => {
  const picked = await tauri.dialog.open({
    multiple: true,
    filters: [{ name: "HEIC images", extensions: ["heic", "heif", "HEIC", "HEIF"] }],
  });
  if (picked) addFiles(Array.isArray(picked) ? picked : [picked]);
});

for (const radio of document.querySelectorAll('input[name="outmode"]')) {
  radio.addEventListener("change", async (e) => {
    if (e.target.value === "custom") {
      const dir = await tauri.dialog.open({ directory: true });
      if (dir) {
        customOutDir = dir;
        outdirLabel.textContent = dir;
        outdirLabel.title = dir;
      } else if (!customOutDir) {
        document.querySelector('input[name="outmode"][value="same"]').checked = true;
      }
    }
  });
}

// ---------- options ----------

qualitySlider.addEventListener("input", () => {
  qualityVal.textContent = qualitySlider.value;
});

formatSel.addEventListener("change", () => {
  qualityRow.style.display = formatSel.value === "jpeg" ? "" : "none";
});

// ---------- context menu toggle (Windows) ----------

const ctxRow = document.getElementById("ctxmenu-row");
const ctxBtn = document.getElementById("ctxmenu-btn");

function renderCtxButton(enabled) {
  ctxBtn.textContent = enabled ? "Enabled ✓ (click to remove)" : "Enable";
  ctxBtn.dataset.enabled = enabled ? "1" : "";
}

(async () => {
  try {
    const status = await tauri.core.invoke("context_menu_status");
    if (status.supported) {
      ctxRow.hidden = false;
      renderCtxButton(status.enabled);
    }
  } catch {}
})();

ctxBtn.addEventListener("click", async () => {
  const target = !ctxBtn.dataset.enabled;
  try {
    await tauri.core.invoke("set_context_menu", { enabled: target });
    renderCtxButton(target);
  } catch (err) {
    summaryEl.textContent = `Error: ${err}`;
  }
});

// ---------- convert ----------

clearBtn.addEventListener("click", () => {
  if (converting) return;
  files.clear();
  fileListEl.replaceChildren();
  refreshUi();
});

tauri.event.listen("conversion-progress", (e) => {
  const { index, total, result } = e.payload;
  const entry = files.get(result.input);
  if (entry) {
    const statusEl = entry.row.querySelector(".file-status");
    if (result.ok) {
      statusEl.textContent = "done ✓";
      statusEl.className = "file-status ok";
    } else {
      statusEl.textContent = result.error || "failed";
      statusEl.className = "file-status err";
      statusEl.title = result.error || "";
    }
  }
  summaryEl.textContent = `Converting… ${index + 1} / ${total}`;
});

convertBtn.addEventListener("click", async () => {
  if (converting || files.size === 0) return;
  converting = true;
  refreshUi();
  convertBtn.textContent = "Converting…";

  const outMode = document.querySelector('input[name="outmode"]:checked').value;
  const paths = [...files.keys()];
  for (const { row } of files.values()) {
    const statusEl = row.querySelector(".file-status");
    statusEl.textContent = "waiting…";
    statusEl.className = "file-status";
  }

  try {
    const results = await tauri.core.invoke("convert_files", {
      files: paths,
      format: formatSel.value,
      quality: Number(qualitySlider.value),
      outDir: outMode === "custom" ? customOutDir : null,
    });
    const ok = results.filter((r) => r.ok).length;
    const failed = results.length - ok;
    summaryEl.textContent =
      failed === 0
        ? `Done — ${ok} file${ok === 1 ? "" : "s"} converted`
        : `Done — ${ok} converted, ${failed} failed`;
  } catch (err) {
    summaryEl.textContent = `Error: ${err}`;
  } finally {
    converting = false;
    convertBtn.textContent = "Convert";
    refreshUi(true); // keep the post-run summary instead of "N files ready"
  }
});
