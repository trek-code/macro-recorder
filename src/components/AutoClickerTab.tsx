import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface AutoClickerConfig {
  interval_min_ms: number;
  interval_max_ms: number;
  click_button: string;
  double_click: boolean;
  position_mode: string;
  fixed_x: number;
  fixed_y: number;
  bbox_x1: number;
  bbox_y1: number;
  bbox_x2: number;
  bbox_y2: number;
  humanize: boolean;
  jitter_px: number;
  stop_after_clicks: number;
  stop_after_seconds: number;
  window_target: string;
}

const DEFAULT_CONFIG: AutoClickerConfig = {
  interval_min_ms: 800,
  interval_max_ms: 1200,
  click_button: "left",
  double_click: false,
  position_mode: "cursor",
  fixed_x: 0,
  fixed_y: 0,
  bbox_x1: 0,
  bbox_y1: 0,
  bbox_x2: 1920,
  bbox_y2: 1080,
  humanize: false,
  jitter_px: 5,
  stop_after_clicks: 0,
  stop_after_seconds: 0,
  window_target: "",
};

interface Props {
  initialConfig?: Partial<AutoClickerConfig>;
  onConfigChange?: (cfg: AutoClickerConfig) => void;
}

export default function AutoClickerTab({ initialConfig, onConfigChange }: Props) {
  const [cfg, setCfg] = useState<AutoClickerConfig>({ ...DEFAULT_CONFIG, ...initialConfig });
  const [running, setRunning] = useState(false);
  const [clicks, setClicks] = useState(0);

  useEffect(() => {
    if (initialConfig) setCfg((c) => ({ ...c, ...initialConfig }));
  }, [initialConfig]);

  useEffect(() => {
    const poll = setInterval(async () => {
      const s = await invoke<{ running: boolean; clicks: number }>("get_autoclicker_status");
      setRunning(s.running);
      setClicks(s.clicks);
    }, 300);
    return () => clearInterval(poll);
  }, []);

  const set = <K extends keyof AutoClickerConfig>(key: K, val: AutoClickerConfig[K]) => {
    setCfg((c) => {
      const next = { ...c, [key]: val };
      onConfigChange?.(next);
      return next;
    });
  };

  async function toggle() {
    if (running) {
      await invoke("stop_autoclicker");
      setRunning(false);
    } else {
      await invoke("start_autoclicker", { config: cfg });
      setRunning(true);
      setClicks(0);
    }
  }

  async function captureWindow() {
    const title = await invoke<string>("get_foreground_window_title");
    set("window_target", title);
  }

  return (
    <div className="tab-layout single">
      <div className="main-panel">
        {/* Start / Stop */}
        <section className="card ac-hero">
          <div className="ac-status">
            {running && <span className="pulse-dot" />}
            <span className="ac-count">{clicks.toLocaleString()}</span>
            <span className="ac-label">clicks</span>
          </div>
          <button className={`btn btn-lg ${running ? "btn-stop" : "btn-play"}`} onClick={toggle}>
            {running ? "⏹ Stop" : "▶ Start"}
          </button>
        </section>

        <div className="two-col">
          {/* Click settings */}
          <section className="card">
            <h2>Click</h2>
            <div className="field-group">
              <label className="option-row">
                <span>Button</span>
                <select className="select" value={cfg.click_button} onChange={(e) => set("click_button", e.target.value)}>
                  <option value="left">Left</option>
                  <option value="right">Right</option>
                  <option value="middle">Middle</option>
                </select>
              </label>
              <label className="option-row">
                <span>Double click</span>
                <input type="checkbox" checked={cfg.double_click} onChange={(e) => set("double_click", e.target.checked)} />
              </label>
            </div>

            <h2 style={{ marginTop: 12 }}>Interval</h2>
            <div className="field-group">
              <label className="option-row">
                <span>Min (ms)</span>
                <input type="number" min="10" max="60000" step="50" className="input input-sm" value={cfg.interval_min_ms}
                  onChange={(e) => set("interval_min_ms", Math.min(Number(e.target.value), cfg.interval_max_ms))} />
              </label>
              <label className="option-row">
                <span>Max (ms)</span>
                <input type="number" min="10" max="60000" step="50" className="input input-sm" value={cfg.interval_max_ms}
                  onChange={(e) => set("interval_max_ms", Math.max(Number(e.target.value), cfg.interval_min_ms))} />
              </label>
            </div>
          </section>

          {/* Position */}
          <section className="card">
            <h2>Position</h2>
            <div className="field-group">
              <label className="option-row">
                <span>Mode</span>
                <select className="select" value={cfg.position_mode} onChange={(e) => set("position_mode", e.target.value)}>
                  <option value="cursor">Cursor (follow mouse)</option>
                  <option value="fixed">Fixed position</option>
                  <option value="bbox">Bounding box (random)</option>
                </select>
              </label>

              {cfg.position_mode === "fixed" && <>
                <label className="option-row">
                  <span>X</span>
                  <input type="number" className="input input-sm" value={cfg.fixed_x} onChange={(e) => set("fixed_x", Number(e.target.value))} />
                </label>
                <label className="option-row">
                  <span>Y</span>
                  <input type="number" className="input input-sm" value={cfg.fixed_y} onChange={(e) => set("fixed_y", Number(e.target.value))} />
                </label>
              </>}

              {cfg.position_mode === "bbox" && <>
                <label className="option-row">
                  <span>X1 · Y1</span>
                  <input type="number" className="input input-xs" value={cfg.bbox_x1} onChange={(e) => set("bbox_x1", Number(e.target.value))} />
                  <input type="number" className="input input-xs" value={cfg.bbox_y1} onChange={(e) => set("bbox_y1", Number(e.target.value))} />
                </label>
                <label className="option-row">
                  <span>X2 · Y2</span>
                  <input type="number" className="input input-xs" value={cfg.bbox_x2} onChange={(e) => set("bbox_x2", Number(e.target.value))} />
                  <input type="number" className="input input-xs" value={cfg.bbox_y2} onChange={(e) => set("bbox_y2", Number(e.target.value))} />
                </label>
              </>}
            </div>
          </section>

          {/* Humanization */}
          <section className="card">
            <h2>Humanization</h2>
            <div className="field-group">
              <label className="option-row">
                <span>Enable</span>
                <input type="checkbox" checked={cfg.humanize} onChange={(e) => set("humanize", e.target.checked)} />
              </label>
              {cfg.humanize && <>
                <label className="option-row">
                  <span>Jitter ±px</span>
                  <input type="range" min="1" max="50" value={cfg.jitter_px} onChange={(e) => set("jitter_px", Number(e.target.value))} />
                  <span className="value-badge">{cfg.jitter_px}px</span>
                </label>
                <p className="hint">Also randomizes hold duration and timing ±10%</p>
              </>}
            </div>
          </section>

          {/* Stop conditions */}
          <section className="card">
            <h2>Stop Conditions</h2>
            <div className="field-group">
              <label className="option-row">
                <span>After clicks</span>
                <input type="number" min="0" className="input input-sm" value={cfg.stop_after_clicks}
                  onChange={(e) => set("stop_after_clicks", Number(e.target.value))} />
                <span className="hint-inline">0 = unlimited</span>
              </label>
              <label className="option-row">
                <span>After seconds</span>
                <input type="number" min="0" className="input input-sm" value={cfg.stop_after_seconds}
                  onChange={(e) => set("stop_after_seconds", Number(e.target.value))} />
                <span className="hint-inline">0 = unlimited</span>
              </label>
              <label className="option-row">
                <span>Window</span>
                <input className="input" value={cfg.window_target} onChange={(e) => set("window_target", e.target.value)} placeholder="Any window" />
                <button className="btn btn-small" onClick={captureWindow}>Capture</button>
              </label>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
