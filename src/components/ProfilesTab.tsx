import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AutoClickerConfig } from "./AutoClickerTab";

export interface Profile {
  name: string;
  playback_speed: number;
  playback_loops: number;
  hotkey_record: string;
  hotkey_play: string;
  hotkey_autoclicker: string;
  autoclicker: AutoClickerConfig;
}

interface Props {
  activeProfileName: string;
  onApply: (p: Profile) => void;
}

const HOTKEY_OPTIONS = [
  "F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12",
  "Ctrl+F1","Ctrl+F2","Ctrl+F9","Ctrl+F10","Ctrl+F11","Ctrl+F12",
  "Alt+F9","Alt+F10","Alt+F11",
];

export default function ProfilesTab({ activeProfileName, onApply }: Props) {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selected, setSelected] = useState<Profile | null>(null);
  const [editing, setEditing] = useState<Profile | null>(null);
  const [newName, setNewName] = useState("");
  const [status, setStatus] = useState("");

  useEffect(() => { load(); }, []);

  async function load() {
    const list = await invoke<Profile[]>("list_profiles");
    setProfiles(list);
    if (!selected) setEditing({ ...list[0] });
  }

  function selectProfile(p: Profile) {
    setSelected(p);
    setEditing({ ...p });
  }

  async function applyProfile(p: Profile) {
    const applied = await invoke<Profile>("apply_profile", { name: p.name });
    onApply(applied);
    setStatus(`Applied "${applied.name}"`);
    // Re-register hotkeys
    await invoke("update_hotkeys", {
      rec: applied.hotkey_record,
      play: applied.hotkey_play,
      ac: applied.hotkey_autoclicker,
    });
  }

  async function saveEdits() {
    if (!editing) return;
    await invoke("save_profile", { profile: editing });
    await applyProfile(editing);
    await load();
    setStatus(`Saved and applied "${editing.name}"`);
  }

  async function createProfile() {
    if (!newName.trim()) return;
    const base = editing ?? profiles[0];
    const p: Profile = { ...base, name: newName.trim() };
    await invoke("save_profile", { profile: p });
    setNewName("");
    await load();
    setStatus(`Created "${p.name}"`);
  }

  async function deleteProfile(p: Profile) {
    if (p.name === "Default") return;
    await invoke("delete_profile", { name: p.name });
    if (selected?.name === p.name) setSelected(null);
    await load();
    setStatus(`Deleted "${p.name}"`);
  }

  const set = <K extends keyof Profile>(key: K, val: Profile[K]) => {
    setEditing((e) => e ? { ...e, [key]: val } : e);
  };

  return (
    <div className="tab-layout">
      <aside className="sidebar">
        <div className="sidebar-title">Profiles</div>
        <ul className="macro-list">
          {profiles.map((p) => (
            <li key={p.name} className={`macro-item ${(selected?.name ?? activeProfileName) === p.name ? "active" : ""}`} onClick={() => selectProfile(p)}>
              <span className="macro-name">{p.name}</span>
              {p.name === activeProfileName && <span className="active-badge">active</span>}
            </li>
          ))}
        </ul>
        <div className="sidebar-footer">
          <input className="input" placeholder="New profile name" value={newName} onChange={(e) => setNewName(e.target.value)} />
          <button className="btn btn-play" onClick={createProfile} disabled={!newName.trim()}>+ Create</button>
        </div>
      </aside>

      <div className="main-panel">
        {editing && (
          <>
            <section className="card">
              <h2>Profile: {editing.name}</h2>
              <div className="two-col">
                {/* Playback defaults */}
                <div>
                  <h3 className="subheading">Playback Defaults</h3>
                  <div className="field-group">
                    <label className="option-row">
                      <span>Speed</span>
                      <input type="range" min="0.25" max="4" step="0.25" value={editing.playback_speed} onChange={(e) => set("playback_speed", Number(e.target.value))} />
                      <span className="value-badge">{editing.playback_speed}x</span>
                    </label>
                    <label className="option-row">
                      <span>Loops</span>
                      <input type="number" min="1" max="9999" className="input input-sm" value={editing.playback_loops} onChange={(e) => set("playback_loops", Number(e.target.value))} />
                    </label>
                  </div>
                </div>

                {/* Hotkeys */}
                <div>
                  <h3 className="subheading">Hotkeys</h3>
                  <div className="field-group">
                    {[
                      { label: "Record toggle", key: "hotkey_record" as const },
                      { label: "Play/Stop",     key: "hotkey_play"   as const },
                      { label: "Auto-clicker",  key: "hotkey_autoclicker" as const },
                    ].map(({ label, key }) => (
                      <label className="option-row" key={key}>
                        <span>{label}</span>
                        <select className="select" value={editing[key] as string} onChange={(e) => set(key, e.target.value)}>
                          <option value="">None</option>
                          {HOTKEY_OPTIONS.map((h) => <option key={h} value={h}>{h}</option>)}
                        </select>
                      </label>
                    ))}
                  </div>
                </div>
              </div>
            </section>

            {/* Auto-clicker defaults */}
            <section className="card">
              <h3 className="subheading">Auto-Clicker Defaults</h3>
              <div className="two-col">
                <div className="field-group">
                  <label className="option-row">
                    <span>Min interval</span>
                    <input type="number" min="10" className="input input-sm" value={editing.autoclicker.interval_min_ms}
                      onChange={(e) => set("autoclicker", { ...editing.autoclicker, interval_min_ms: Number(e.target.value) })} />
                    <span className="hint-inline">ms</span>
                  </label>
                  <label className="option-row">
                    <span>Max interval</span>
                    <input type="number" min="10" className="input input-sm" value={editing.autoclicker.interval_max_ms}
                      onChange={(e) => set("autoclicker", { ...editing.autoclicker, interval_max_ms: Number(e.target.value) })} />
                    <span className="hint-inline">ms</span>
                  </label>
                  <label className="option-row">
                    <span>Button</span>
                    <select className="select" value={editing.autoclicker.click_button}
                      onChange={(e) => set("autoclicker", { ...editing.autoclicker, click_button: e.target.value })}>
                      <option value="left">Left</option>
                      <option value="right">Right</option>
                      <option value="middle">Middle</option>
                    </select>
                  </label>
                </div>
                <div className="field-group">
                  <label className="option-row">
                    <span>Position mode</span>
                    <select className="select" value={editing.autoclicker.position_mode}
                      onChange={(e) => set("autoclicker", { ...editing.autoclicker, position_mode: e.target.value })}>
                      <option value="cursor">Cursor</option>
                      <option value="fixed">Fixed</option>
                      <option value="bbox">Bounding box</option>
                    </select>
                  </label>
                  <label className="option-row">
                    <span>Humanize</span>
                    <input type="checkbox" checked={editing.autoclicker.humanize}
                      onChange={(e) => set("autoclicker", { ...editing.autoclicker, humanize: e.target.checked })} />
                  </label>
                </div>
              </div>
            </section>

            <div className="row gap">
              <button className="btn btn-play" onClick={saveEdits}>Save &amp; Apply</button>
              {editing.name !== "Default" && (
                <button className="btn btn-danger" onClick={() => deleteProfile(editing)}>Delete Profile</button>
              )}
            </div>
          </>
        )}

        {status && <div className="status-bar">{status}</div>}
      </div>
    </div>
  );
}
