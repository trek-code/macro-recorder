import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScheduledTask {
  id: string;
  macro_name: string;
  schedule: string;
  speed: number;
  loops: number;
  enabled: boolean;
  last_run_ms: number | null;
}

interface MacroData { name: string }

type ScheduleType = "every_seconds" | "every_minutes" | "daily" | "once";

function parseScheduleDisplay(s: string): string {
  if (s.startsWith("every_") && s.endsWith("_seconds")) {
    return `Every ${s.replace("every_", "").replace("_seconds", "").replace(/_/g, " ")}s`;
  }
  if (s.startsWith("every_") && s.endsWith("_minutes")) {
    return `Every ${s.replace("every_", "").replace("_minutes", "").replace(/_/g, " ")}m`;
  }
  if (s.startsWith("daily_")) return `Daily at ${s.replace("daily_", "")}`;
  if (s.startsWith("once_")) {
    const ms = parseInt(s.replace("once_", ""));
    return `Once at ${new Date(ms).toLocaleString()}`;
  }
  return s;
}

function buildSchedule(type: ScheduleType, value: string): string {
  switch (type) {
    case "every_seconds": return `every_${value}_seconds`;
    case "every_minutes": return `every_${value}_minutes`;
    case "daily": return `daily_${value}`;
    case "once": return `once_${Date.now() + Number(value) * 1000}`;
  }
}

export default function SchedulerTab() {
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [macros, setMacros] = useState<MacroData[]>([]);
  const [status, setStatus] = useState("");

  // New task form
  const [macroName, setMacroName] = useState("");
  const [scheduleType, setScheduleType] = useState<ScheduleType>("every_seconds");
  const [scheduleValue, setScheduleValue] = useState("60");
  const [speed, setSpeed] = useState(1.0);
  const [loops, setLoops] = useState(1);

  useEffect(() => {
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, []);

  async function load() {
    const [t, m] = await Promise.all([
      invoke<ScheduledTask[]>("list_scheduled_tasks"),
      invoke<MacroData[]>("list_macros"),
    ]);
    setTasks(t);
    setMacros(m);
    if (!macroName && m.length > 0) setMacroName(m[0].name);
  }

  async function addTask() {
    if (!macroName) return setStatus("Select a macro first.");
    const schedule = buildSchedule(scheduleType, scheduleValue);
    try {
      await invoke("add_scheduled_task", { macroName, schedule, speed, loops });
      setStatus("Task scheduled.");
      load();
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function toggleTask(id: string, enabled: boolean) {
    await invoke("toggle_scheduled_task", { id, enabled: !enabled });
    load();
  }

  async function deleteTask(id: string) {
    await invoke("delete_scheduled_task", { id });
    load();
  }

  async function runNow(id: string) {
    setStatus("Running task…");
    try {
      await invoke("run_task_now", { id });
      setStatus("Task ran successfully.");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  const scheduleHint: Record<ScheduleType, string> = {
    every_seconds: "Interval in seconds (e.g. 30)",
    every_minutes: "Interval in minutes (e.g. 5)",
    daily: "Time in HH:MM format (e.g. 09:00)",
    once: "Delay in seconds from now",
  };

  return (
    <div className="tab-layout single-col">
      <section className="card">
        <h2>Schedule New Task</h2>
        <div className="form-grid">
          <label className="option-row">
            <span>Macro</span>
            <select className="input" value={macroName} onChange={(e) => setMacroName(e.target.value)}>
              {macros.map((m) => <option key={m.name}>{m.name}</option>)}
              {macros.length === 0 && <option value="">— No macros —</option>}
            </select>
          </label>
          <label className="option-row">
            <span>Schedule</span>
            <select className="input" value={scheduleType} onChange={(e) => setScheduleType(e.target.value as ScheduleType)}>
              <option value="every_seconds">Every N seconds</option>
              <option value="every_minutes">Every N minutes</option>
              <option value="daily">Daily at time</option>
              <option value="once">Once (delay)</option>
            </select>
          </label>
          <label className="option-row">
            <span>{scheduleHint[scheduleType]}</span>
            <input
              className="input"
              value={scheduleValue}
              onChange={(e) => setScheduleValue(e.target.value)}
              placeholder={scheduleType === "daily" ? "09:00" : "60"}
            />
          </label>
          <label className="option-row">
            <span>Speed</span>
            <input type="range" min="0.25" max="4" step="0.25" value={speed} onChange={(e) => setSpeed(Number(e.target.value))} />
            <span className="value-badge">{speed}x</span>
          </label>
          <label className="option-row">
            <span>Loops</span>
            <input type="number" min="1" max="9999" value={loops} onChange={(e) => setLoops(Math.max(1, Number(e.target.value)))} className="input input-sm" />
          </label>
        </div>
        <button className="btn btn-play" onClick={addTask}>+ Add Task</button>
      </section>

      <section className="card">
        <h2>Scheduled Tasks ({tasks.length})</h2>
        {tasks.length === 0 && <p className="empty-hint">No tasks scheduled.</p>}
        <div className="task-list">
          {tasks.map((t) => (
            <div key={t.id} className={`task-row ${t.enabled ? "enabled" : "disabled"}`}>
              <div className="task-info">
                <span className="task-name">{t.macro_name}</span>
                <span className="task-schedule">{parseScheduleDisplay(t.schedule)}</span>
                {t.last_run_ms && (
                  <span className="task-last">Last: {new Date(t.last_run_ms).toLocaleString()}</span>
                )}
              </div>
              <div className="task-actions row gap">
                <button className="btn btn-small" onClick={() => runNow(t.id)}>▶ Now</button>
                <button
                  className={`btn btn-small ${t.enabled ? "btn-secondary" : "btn-play"}`}
                  onClick={() => toggleTask(t.id, t.enabled)}
                >
                  {t.enabled ? "Pause" : "Enable"}
                </button>
                <button className="btn btn-danger btn-small" onClick={() => deleteTask(t.id)}>Delete</button>
              </div>
            </div>
          ))}
        </div>
      </section>

      {status && <div className="status-bar">{status}</div>}
    </div>
  );
}
