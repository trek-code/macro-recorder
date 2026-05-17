import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export interface MacroEvent {
  timestamp_ms: number;
  event: { type: string; data: Record<string, unknown> };
}

export interface MacroData {
  name: string;
  events: MacroEvent[];
  duration_ms: number;
  tags: string[];
  description: string;
  relative_window: string | null;
}

export interface ChainItem {
  macro_name: string;
  loops: number;
  delay_after_ms: number;
}

interface Props {
  macros: MacroData[];
  recording: boolean;
  playing: boolean;
  paused: boolean;
  speed: number;
  loops: number;
  onMacrosChange: () => void;
  onSpeedChange: (v: number) => void;
  onLoopsChange: (v: number) => void;
}

function fmt(ms: number) {
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
}

export default function MacrosTab({
  macros,
  recording,
  playing,
  paused,
  speed,
  loops,
  onMacrosChange,
  onSpeedChange,
  onLoopsChange,
}: Props) {
  const [selected, setSelected] = useState<MacroData | null>(null);
  const [newName, setNewName] = useState("My Macro");
  const [elapsed, setElapsed] = useState(0);
  const [windowTarget, setWindowTarget] = useState("");
  const [relativeMode, setRelativeMode] = useState(false);
  const [status, setStatus] = useState("");
  const [editMode, setEditMode] = useState(false);
  const [editEvents, setEditEvents] = useState<MacroEvent[]>([]);
  const [chain, setChain] = useState<ChainItem[]>([]);
  const [chainPlaying, setChainPlaying] = useState(false);
  const [search, setSearch] = useState("");
  const [tagFilter, setTagFilter] = useState("");
  const timerRef = useRef<number | null>(null);

  // Edit meta
  const [editingMeta, setEditingMeta] = useState(false);
  const [metaTags, setMetaTags] = useState("");
  const [metaDesc, setMetaDesc] = useState("");

  const filteredMacros = macros.filter((m) => {
    const nameMatch = m.name.toLowerCase().includes(search.toLowerCase());
    const tagMatch = !tagFilter || m.tags?.includes(tagFilter);
    return nameMatch && tagMatch;
  });

  const allTags = Array.from(new Set(macros.flatMap((m) => m.tags ?? [])));

  async function startRecording() {
    setElapsed(0);
    if (relativeMode) {
      await invoke("start_recording_relative");
    } else {
      await invoke("start_recording");
    }
    const t0 = Date.now();
    timerRef.current = window.setInterval(() => setElapsed(Date.now() - t0), 100);
  }

  async function stopRecording() {
    if (timerRef.current) clearInterval(timerRef.current);
    const result = await invoke<MacroData>("stop_recording", { name: newName });
    await invoke("save_macro", { macroData: result });
    setStatus(`Recorded ${result.events.length} events (${fmt(result.duration_ms)})`);
    setSelected(result);
    onMacrosChange();
  }

  async function pauseRecording() {
    await invoke("pause_recording");
  }

  async function resumeRecording() {
    await invoke("resume_recording");
  }

  async function playMacro(m: MacroData) {
    setStatus(`Playing "${m.name}"…`);
    try {
      await invoke("play_macro", { macroData: m, speed, loops, windowTarget });
      setStatus(`Done playing "${m.name}"`);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
    onMacrosChange();
  }

  async function stopPlayback() {
    await invoke("stop_playback");
    setStatus("Stopped");
  }

  async function deleteMacro(m: MacroData) {
    await invoke("delete_macro", { name: m.name });
    if (selected?.name === m.name) setSelected(null);
    onMacrosChange();
    setStatus(`Deleted "${m.name}"`);
  }

  async function compressMacro(m: MacroData) {
    const compressed = await invoke<MacroData>("compress_macro", { macroData: m });
    await invoke("save_macro", { macroData: compressed });
    setStatus(`Compressed: ${m.events.length} → ${compressed.events.length} events`);
    onMacrosChange();
  }

  async function captureForeground() {
    const title = await invoke<string>("get_foreground_window_title");
    setWindowTarget(title);
  }

  async function handleImport() {
    const path = await open({ filters: [{ name: "Macro", extensions: ["json"] }] });
    if (!path || typeof path !== "string") return;
    const m = await invoke<MacroData>("import_macro", { path });
    setStatus(`Imported "${m.name}"`);
    onMacrosChange();
  }

  async function handleExport(m: MacroData) {
    const path = await save({
      defaultPath: `${m.name}.json`,
      filters: [{ name: "Macro", extensions: ["json"] }],
    });
    if (!path) return;
    await invoke("export_macro", { name: m.name, path });
    setStatus(`Exported "${m.name}"`);
  }

  function openStepEditor(m: MacroData) {
    setEditEvents([...m.events]);
    setEditMode(true);
  }

  async function saveStepEdits() {
    if (!selected) return;
    await invoke("update_macro_events", { name: selected.name, events: editEvents });
    onMacrosChange();
    setEditMode(false);
    setStatus("Step edits saved");
  }

  function deleteStep(i: number) {
    setEditEvents((ev) => ev.filter((_, idx) => idx !== i));
  }

  function openMetaEditor(m: MacroData) {
    setMetaTags((m.tags ?? []).join(", "));
    setMetaDesc(m.description ?? "");
    setEditingMeta(true);
  }

  async function saveMetaEdits() {
    if (!selected) return;
    const tags = metaTags.split(",").map((t) => t.trim()).filter(Boolean);
    await invoke("update_macro_meta", { name: selected.name, tags, description: metaDesc });
    onMacrosChange();
    setEditingMeta(false);
    setStatus("Metadata saved");
  }

  function addToChain(m: MacroData) {
    setChain((c) => [...c, { macro_name: m.name, loops: 1, delay_after_ms: 0 }]);
  }

  function removeFromChain(i: number) {
    setChain((c) => c.filter((_, idx) => idx !== i));
  }

  function updateChainItem(i: number, field: keyof ChainItem, value: number) {
    setChain((c) => c.map((item, idx) => idx === i ? { ...item, [field]: value } : item));
  }

  async function runChain() {
    if (chain.length === 0) return;
    setChainPlaying(true);
    setStatus("Running chain…");
    try {
      await invoke("run_chain", { chain, speed, windowTarget });
      setStatus("Chain complete");
    } catch (e) {
      setStatus(`Chain error: ${e}`);
    }
    setChainPlaying(false);
  }

  async function stopChain() {
    await invoke("stop_playback");
    setChainPlaying(false);
    setStatus("Chain stopped");
  }

  return (
    <div className="tab-layout">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-title">
          <span>Macros ({filteredMacros.length})</span>
          <button className="icon-btn" title="Import macro" onClick={handleImport}>↑</button>
        </div>
        <input
          className="input search-input"
          placeholder="Search…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {allTags.length > 0 && (
          <div className="tag-filter-row">
            <button className={`tag-chip sm ${!tagFilter ? "active" : ""}`} onClick={() => setTagFilter("")}>All</button>
            {allTags.map((t) => (
              <button key={t} className={`tag-chip sm ${tagFilter === t ? "active" : ""}`} onClick={() => setTagFilter(t === tagFilter ? "" : t)}>
                {t}
              </button>
            ))}
          </div>
        )}
        {filteredMacros.length === 0 && <p className="empty-hint">No macros.</p>}
        <ul className="macro-list">
          {filteredMacros.map((m) => (
            <li
              key={m.name}
              className={`macro-item ${selected?.name === m.name ? "active" : ""}`}
              onClick={() => { setSelected(m); setEditMode(false); setEditingMeta(false); }}
            >
              <span className="macro-name">{m.name}</span>
              <span className="macro-meta">{m.events.length} ev · {fmt(m.duration_ms)}</span>
              {(m.tags ?? []).length > 0 && (
                <span className="macro-tags">{m.tags.join(", ")}</span>
              )}
            </li>
          ))}
        </ul>
      </aside>

      <div className="main-panel">
        {/* Step editor overlay */}
        {editMode && selected && (
          <section className="card">
            <div className="card-header">
              <h2>Step Editor — {selected.name}</h2>
              <div className="row gap">
                <button className="btn btn-play" onClick={saveStepEdits}>Save</button>
                <button className="btn btn-stop" onClick={() => setEditMode(false)}>Cancel</button>
              </div>
            </div>
            <p className="macro-stats">{editEvents.length} events</p>
            <div className="event-log">
              {editEvents.map((ev, i) => (
                <div key={i} className="event-row">
                  <span className="event-ts">{ev.timestamp_ms}ms</span>
                  <span className={`event-badge type-${ev.event.type.toLowerCase()}`}>{ev.event.type}</span>
                  <button className="icon-btn danger" onClick={() => deleteStep(i)}>✕</button>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Meta editor overlay */}
        {editingMeta && selected && (
          <section className="card">
            <div className="card-header">
              <h2>Tags & Description — {selected.name}</h2>
              <div className="row gap">
                <button className="btn btn-play" onClick={saveMetaEdits}>Save</button>
                <button className="btn btn-stop" onClick={() => setEditingMeta(false)}>Cancel</button>
              </div>
            </div>
            <label className="option-row">
              <span>Tags</span>
              <input className="input" value={metaTags} onChange={(e) => setMetaTags(e.target.value)} placeholder="gaming, productivity, …" />
            </label>
            <label className="option-row">
              <span>Description</span>
              <input className="input" value={metaDesc} onChange={(e) => setMetaDesc(e.target.value)} placeholder="What does this macro do?" />
            </label>
          </section>
        )}

        {!editMode && !editingMeta && <>
          {/* Record */}
          <section className="card">
            <h2>Record</h2>
            <div className="row gap">
              <input className="input" value={newName} onChange={(e) => setNewName(e.target.value)} disabled={recording} placeholder="Macro name" />
              {!recording
                ? <>
                  <button className="btn btn-record" onClick={startRecording} disabled={playing}>⏺ Record</button>
                  <label className="option-row compact">
                    <input type="checkbox" checked={relativeMode} onChange={(e) => setRelativeMode(e.target.checked)} />
                    <span>Relative window</span>
                  </label>
                </>
                : <>
                  <button className="btn btn-stop" onClick={stopRecording}>⏹ Stop ({fmt(elapsed)})</button>
                  {!paused
                    ? <button className="btn btn-secondary" onClick={pauseRecording}>⏸ Pause</button>
                    : <button className="btn btn-play" onClick={resumeRecording}>▶ Resume</button>
                  }
                </>
              }
            </div>
            {recording && <div className="recording-indicator"><span className="pulse-dot" /> {paused ? "Paused" : "Recording…"}</div>}
          </section>

          {/* Playback */}
          <section className="card">
            <h2>Playback</h2>
            <div className="playback-options">
              <label className="option-row">
                <span>Speed</span>
                <input type="range" min="0.25" max="4" step="0.25" value={speed} onChange={(e) => onSpeedChange(Number(e.target.value))} />
                <span className="value-badge">{speed}x</span>
              </label>
              <label className="option-row">
                <span>Loops</span>
                <input type="number" min="1" max="9999" value={loops} onChange={(e) => onLoopsChange(Math.max(1, Number(e.target.value)))} className="input input-sm" />
              </label>
              <label className="option-row">
                <span>Window</span>
                <input className="input" value={windowTarget} onChange={(e) => setWindowTarget(e.target.value)} placeholder="Any window" />
                <button className="btn btn-small" onClick={captureForeground} title="Use active window">Capture</button>
              </label>
            </div>
            <div className="row gap">
              <button className="btn btn-play" disabled={!selected || playing || recording} onClick={() => selected && playMacro(selected)}>
                ▶ {playing ? "Playing…" : selected ? `Play "${selected.name}"` : "Play"}
              </button>
              {playing && <button className="btn btn-stop" onClick={stopPlayback}>⏹ Stop</button>}
              {selected && !playing && !recording && <>
                <button className="btn btn-secondary" onClick={() => openStepEditor(selected)}>Edit Steps</button>
                <button className="btn btn-secondary" onClick={() => openMetaEditor(selected)}>Tags</button>
                <button className="btn btn-secondary" onClick={() => compressMacro(selected)}>Compress</button>
                <button className="btn btn-secondary" onClick={() => handleExport(selected)}>Export</button>
                <button className="btn btn-danger" onClick={() => deleteMacro(selected)}>Delete</button>
              </>}
            </div>
            {selected && !playing && !recording && (
              <button className="btn btn-small chain-add" onClick={() => addToChain(selected)}>
                + Add "{selected.name}" to Chain
              </button>
            )}
          </section>

          {/* Chain editor */}
          {chain.length > 0 && (
            <section className="card">
              <h2>Chain</h2>
              <div className="chain-list">
                {chain.map((item, i) => (
                  <div key={i} className="chain-row">
                    <span className="chain-name">{item.macro_name}</span>
                    <label className="option-row compact">
                      <span>×</span>
                      <input type="number" min="1" max="999" value={item.loops} onChange={(e) => updateChainItem(i, "loops", Number(e.target.value))} className="input input-xs" />
                    </label>
                    <label className="option-row compact">
                      <span>Delay</span>
                      <input type="number" min="0" max="60000" step="100" value={item.delay_after_ms} onChange={(e) => updateChainItem(i, "delay_after_ms", Number(e.target.value))} className="input input-xs" />
                      <span>ms</span>
                    </label>
                    <button className="icon-btn danger" onClick={() => removeFromChain(i)}>✕</button>
                  </div>
                ))}
              </div>
              <div className="row gap">
                {!chainPlaying
                  ? <button className="btn btn-play" onClick={runChain}>▶ Run Chain</button>
                  : <button className="btn btn-stop" onClick={stopChain}>⏹ Stop Chain</button>
                }
                <button className="btn btn-danger" onClick={() => setChain([])}>Clear</button>
              </div>
            </section>
          )}

          {/* Selected macro detail */}
          {selected && (
            <section className="card">
              <h2>{selected.name}</h2>
              {selected.description && <p className="muted">{selected.description}</p>}
              {(selected.tags ?? []).length > 0 && (
                <div className="tag-filter-row">{selected.tags.map((t) => <span key={t} className="tag-chip sm">{t}</span>)}</div>
              )}
              {selected.relative_window && <p className="muted">Relative: {selected.relative_window}</p>}
              <p className="macro-stats">{selected.events.length} events · {fmt(selected.duration_ms)}</p>
              <div className="event-log">
                {selected.events.slice(0, 40).map((ev, i) => (
                  <div key={i} className="event-row">
                    <span className="event-ts">{ev.timestamp_ms}ms</span>
                    <span className={`event-badge type-${ev.event.type.toLowerCase()}`}>{ev.event.type}</span>
                  </div>
                ))}
                {selected.events.length > 40 && <p className="event-more">+{selected.events.length - 40} more</p>}
              </div>
            </section>
          )}
        </>}

        {status && <div className="status-bar">{status}</div>}
      </div>
    </div>
  );
}
