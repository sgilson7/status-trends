// The page draws what the core returns and decides nothing.
// Every call goes through the shim, which returns {"ok": ...} or {"error": "..."}.
//
// Privacy: the data a person opens is read with the browser's own file
// reader and handed to the core as a string. Nothing here sends it, stores
// it or keeps it past the tab. Every file this page needs is fetched in
// start(), before the page reports ready; after that the page makes no
// request at all, which the browser check verifies in three engines.
import init, { load, analyze } from "./pkg/tool_shim.js";

const el = (id) => document.getElementById(id);
const SVG_NS = el("svg-template").content.firstElementChild.namespaceURI;
const ROLES = ["created", "status", "resolved", "due", "group"];
const GROUPS = ["requested", "pending", "done", "excluded"];
const KEPT = ["granularity", "fiscal_start", "show", "vocabulary", "target", "growth", "aging"];

let strings = null;
let stringsText = "";
let exportCss = "";
const samples = {};
let source = null;
let settings = fresh({});
let view = null;

function fresh(keep) {
  const s = {
    granularity: "quarter", fiscal_start: "1", show: "all", vocabulary: "requests",
    created: "", status: "", resolved: "", due: "", group: "", date_order: "",
    as_of: "", target: "", growth: "", aging: "", only_group: "", statuses: {},
  };
  KEPT.forEach((k) => {
    if (keep[k] !== undefined) {
      s[k] = keep[k];
    }
  });
  return s;
}

function call(fn, ...args) {
  return JSON.parse(fn(...args));
}

function make(tag, className, text) {
  const node = document.createElement(tag);
  if (className) {
    node.className = className;
  }
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

function option(value, text) {
  const o = make("option", "", text);
  o.value = value;
  return o;
}

function showMessage(text) {
  el("message").textContent = text;
}

// ---- loading data ------------------------------------------------------

function open(name, text) {
  source = { name, text };
  settings = fresh(settings);
  el("loaded-name").textContent = name;
  el("loaded").hidden = false;
  run();
}

function openSample(key, name, extra) {
  settings = fresh(settings);
  Object.assign(settings, extra);
  source = { name, text: samples[key] };
  el("loaded-name").textContent = name;
  el("loaded").hidden = false;
  run();
}

async function openFile(file) {
  if (!file) {
    return;
  }
  open(file.name, await file.text());
}

function clearAll() {
  source = null;
  view = null;
  settings = fresh(settings);
  el("file").value = "";
  el("paste").value = "";
  el("loaded").hidden = true;
  el("loaded-name").textContent = "";
  el("dashboard").hidden = true;
  el("empty").hidden = false;
  el("charts").replaceChildren();
  el("period-table").replaceChildren();
  el("group-table").replaceChildren();
  showMessage("");
  hideTip();
}

// ---- the one call into the core ---------------------------------------

function run() {
  hideTip();
  if (!source) {
    return;
  }
  const result = call(analyze, source.text, JSON.stringify(settings), stringsText);
  if (result.error) {
    view = null;
    showMessage(result.error);
    el("dashboard").hidden = true;
    el("empty").hidden = false;
    return;
  }
  showMessage("");
  view = result.ok;
  draw();
}

function set(key, value) {
  settings[key] = value;
  run();
}

// ---- drawing -----------------------------------------------------------

function draw() {
  el("empty").hidden = true;
  el("dashboard").hidden = false;
  drawControls();
  drawSettings();
  const problem = view.problem;
  el("problem").hidden = !problem;
  el("problem").textContent = problem || "";
  el("results").hidden = Boolean(problem);
  if (problem) {
    el("settings").open = true;
    return;
  }
  el("summary").textContent = view.summary.text;
  el("notes").replaceChildren(...view.summary.notes.map((n) => make("li", "", n)));
  drawTiles();
  drawInsights();
  drawCharts();
  drawTable(el("period-table"), view.table);
  el("group-section").hidden = !view.group_table;
  if (view.group_table) {
    drawTable(el("group-table"), view.group_table);
  }
}

function drawControls() {
  ["granularity", "fiscal_start", "show"].forEach((k) => {
    el(k).value = settings[k];
  });
  el("vocabulary").value = settings.vocabulary;
  const only = el("only_group");
  const all = option("", strings.ui.only_all);
  only.replaceChildren(all, ...view.groups.map((g) => option(g, g)));
  only.value = settings.only_group;
  el("only-control").hidden = view.groups.length === 0 && settings.only_group === "";
}

function drawSettings() {
  ROLES.forEach((role) => {
    const select = el(role);
    const choices = [option("none", strings.ui.none_option)];
    view.columns.forEach((c) => choices.push(option(String(c.index), c.name)));
    select.replaceChildren(...choices);
    const chosen = view.roles[role];
    select.value = chosen === null ? "none" : String(chosen);
  });
  el("date_order").value = view.date_order;
  ["as_of", "target", "growth", "aging"].forEach((k) => {
    if (document.activeElement !== el(k)) {
      el(k).value = settings[k];
    }
  });
  const words = vocabulary();
  el("statuses").replaceChildren(...view.statuses.map((s) => {
    const row = make("tr", s.moved ? "moved" : "");
    const select = make("select");
    select.setAttribute("aria-label", s.name);
    GROUPS.forEach((g) => select.append(option(g, words[g])));
    select.value = s.group;
    select.addEventListener("change", () => {
      settings.statuses[s.name] = select.value;
      run();
    });
    const cell = make("td");
    cell.append(select);
    row.append(make("td", "", s.name), make("td", "num", String(s.count)), cell);
    return row;
  }));
}

function vocabulary() {
  return strings.vocabularies.find((v) => v.key === settings.vocabulary) || strings.vocabularies[0];
}

function drawTiles() {
  el("tiles").replaceChildren(...view.tiles.map((t, i) => {
    const tile = make("div", i === 0 ? "tile hero-tile" : "tile");
    tile.append(make("p", "tile-label", t.label), make("p", "tile-value", t.value));
    if (t.deltas.length) {
      const list = make("p", "tile-deltas");
      t.deltas.forEach((d) => list.append(make("span", "delta " + d.tone, d.text)));
      tile.append(list);
    }
    tile.append(make("p", "tile-detail", t.detail));
    return tile;
  }));
}

function drawInsights() {
  el("insights").replaceChildren(...view.insights.map((i) => {
    const item = make("li", "insight " + i.level);
    item.append(
      make("span", "insight-icon", ""),
      make("span", "insight-level", strings.ui["level_" + i.level]),
      make("span", "insight-text", i.text),
    );
    return item;
  }));
}

function svg(tag) {
  return document.createElementNS(SVG_NS, tag);
}

function drawMark(m, chart) {
  const node = svg(m.tag);
  m.attrs.forEach(([k, v]) => node.setAttribute(k, v));
  if (m.class) {
    node.setAttribute("class", m.class);
  }
  if (m.text) {
    node.textContent = m.text;
  }
  if (m.tip !== undefined) {
    const lines = chart.tips[m.tip];
    node.setAttribute("aria-label", lines.join(", "));
    node.addEventListener("pointerenter", (e) => showTip(lines, e.clientX, e.clientY));
    node.addEventListener("pointermove", (e) => moveTip(e.clientX, e.clientY));
    node.addEventListener("pointerleave", hideTip);
    node.addEventListener("focus", () => {
      const r = node.getBoundingClientRect();
      showTip(lines, r.left, r.top);
    });
    node.addEventListener("blur", hideTip);
  }
  (m.children || []).forEach((c) => node.append(drawMark(c, chart)));
  return node;
}

function drawCharts() {
  el("charts").replaceChildren(...view.charts.map((c) => {
    const figure = make("figure", "chart");
    figure.dataset.key = c.key;
    const head = make("div", "chart-head");
    head.append(make("h3", "", c.title));
    const save = make("button", "small", strings.ui.save_svg);
    save.type = "button";
    head.append(save);
    figure.append(head, make("p", "chart-note", c.note));
    if (c.legend.length) {
      const legend = make("ul", "legend");
      c.legend.forEach((l) => {
        const item = make("li");
        item.append(make("span", "key " + l.class), make("span", "", l.label));
        legend.append(item);
      });
      figure.append(legend);
    }
    const root = el("svg-template").content.firstElementChild.cloneNode(true);
    root.setAttribute("viewBox", ["0", "0", c.width, c.height].join(" "));
    root.setAttribute("aria-label", c.aria);
    c.marks.forEach((m) => root.append(drawMark(m, c)));
    figure.append(root);
    save.addEventListener("click", () => saveSvg(root, c));
    return figure;
  }));
}

function drawTable(table, t) {
  const head = make("thead");
  const hr = make("tr");
  t.headers.forEach((h) => hr.append(make("th", "", h)));
  head.append(hr);
  const body = make("tbody");
  t.rows.forEach((r, i) => {
    const tr = make("tr", t.partial[i] ? "partial" : "");
    r.forEach((cell, j) => tr.append(make(j === 0 ? "th" : "td", j === 0 ? "" : "num", cell)));
    body.append(tr);
  });
  table.replaceChildren(head, body);
}

// ---- tooltip -----------------------------------------------------------

function showTip(lines, x, y) {
  const tip = el("tip");
  tip.replaceChildren(...lines.map((l, i) => make(i === 0 ? "strong" : "span", "", l)));
  tip.hidden = false;
  moveTip(x, y);
}

function moveTip(x, y) {
  const tip = el("tip");
  tip.style.left = x + "px";
  tip.style.top = y + "px";
}

function hideTip() {
  el("tip").hidden = true;
}

// ---- saving files on the person's own device ----------------------------

function download(name, type, text) {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = make("a");
  a.href = url;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

function saveSvg(root, chart) {
  const copy = root.cloneNode(true);
  const style = svg("style");
  style.textContent = exportCss;
  copy.prepend(style);
  copy.setAttribute("class", "chart-svg export");
  const title = svg("title");
  title.textContent = chart.title;
  copy.prepend(title);
  download("status-trends-" + chart.key + ".svg", "image/svg+xml", new XMLSerializer().serializeToString(copy));
}

// ---- wiring --------------------------------------------------------------

function wire() {
  el("file").addEventListener("change", () => openFile(el("file").files[0]));
  const drop = el("drop");
  // A file dropped anywhere is opened here, never by the browser as a page.
  ["dragover", "drop"].forEach((t) => window.addEventListener(t, (e) => e.preventDefault()));
  drop.addEventListener("dragover", () => drop.classList.add("over"));
  drop.addEventListener("dragleave", () => drop.classList.remove("over"));
  window.addEventListener("drop", (e) => {
    drop.classList.remove("over");
    openFile(e.dataTransfer.files[0]);
  });
  el("use-paste").addEventListener("click", () => open(strings.ui.paste_label, el("paste").value));
  el("sample-requests").addEventListener("click", () =>
    openSample("requests", strings.ui.sample_requests, { vocabulary: "requests", granularity: "quarter", fiscal_start: "1" }));
  el("sample-class").addEventListener("click", () =>
    openSample("class", strings.ui.sample_class, { vocabulary: "class", granularity: "month", fiscal_start: "8" }));
  el("clear").addEventListener("click", clearAll);

  ["granularity", "fiscal_start", "show", "vocabulary", "only_group", "date_order"].forEach((k) =>
    el(k).addEventListener("change", () => set(k, el(k).value)));
  ROLES.forEach((role) => el(role).addEventListener("change", () => {
    settings.only_group = role === "group" ? "" : settings.only_group;
    set(role, el(role).value);
  }));
  ["as_of", "target", "growth", "aging"].forEach((k) =>
    el(k).addEventListener("change", () => set(k, el(k).value)));
  el("save-csv").addEventListener("click", () => download(view.export_name, "text/csv", view.export_csv));
  el("print").addEventListener("click", () => window.print());
}

async function text(path) {
  const response = await fetch(path);
  if (!response.ok) {
    throw new Error(path + ": " + response.status);
  }
  return response.text();
}

async function pageLoaded() {
  if (document.readyState !== "complete") {
    await new Promise((resolve) => window.addEventListener("load", resolve, { once: true }));
  }
}

async function start() {
  await init();
  // Every file the page will ever need, fetched now and kept in memory.
  const [s, requests, cls, css] = await Promise.all([
    text("data/strings.json"),
    text("data/sample-requests.csv"),
    text("data/sample-class.csv"),
    text("data/chart-export.css"),
  ]);
  const result = call(load, s);
  if (result.error) {
    showMessage(result.error);
    document.body.dataset.ready = "true";
    return;
  }
  strings = result.ok;
  stringsText = s;
  samples.requests = requests;
  samples.class = cls;
  exportCss = css;
  document.title = strings.ui.title;
  document.querySelectorAll("[data-text]").forEach((node) => {
    node.textContent = strings.ui[node.dataset.text];
  });
  el("source").href = strings.ui.source_url;
  el("vocabulary").replaceChildren(...strings.vocabularies.map((v) => option(v.key, v.name)));
  wire();
  await pageLoaded();
  document.body.dataset.ready = "true";
}

start();
