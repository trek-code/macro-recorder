import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const STARTER = `// Rhai script — full automation API available
// click(x, y)  right_click(x, y)  double_click(x, y)
// type_text("hello {date}")  key("Return")  hotkey("ctrl", "c")
// sleep(ms)  log("message")
// get_pixel(x, y) => [r, g, b]
// wait_pixel(x, y, r, g, b, tolerance, timeout_ms) => bool
// find_image(b64, confidence) => [found, cx, cy]
// wait_image(b64, confidence, timeout_ms) => [found, cx, cy]
// click_image(b64, confidence, timeout_ms) => bool
// get_var("name")  set_var("name", "value")
// get_window_title()  window_matches("Notepad")

click(100, 200);
sleep(500);
type_text("Hello from Rhai!");
log("Done!");
`;

export default function ScriptTab() {
  const [script, setScript] = useState(STARTER);
  const [macroName, setMacroName] = useState("MyScript");
  const [output, setOutput] = useState<string[]>([]);
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState("");

  async function runScript() {
    setRunning(true);
    setOutput([]);
    setStatus("Running…");
    try {
      const lines = await invoke<string[]>("run_script", { script });
      setOutput(lines);
      setStatus("Script completed successfully.");
    } catch (e) {
      const msg = String(e);
      setOutput(msg.split("\n"));
      setStatus("Script error — see output.");
    }
    setRunning(false);
  }

  async function saveScriptMacro() {
    try {
      await invoke("save_script_macro", { name: macroName, script });
      setStatus(`Saved as macro "${macroName}"`);
    } catch (e) {
      setStatus(`Save failed: ${e}`);
    }
  }

  return (
    <div className="tab-layout single-col">
      <section className="card">
        <div className="card-header">
          <h2>Script Editor (Rhai)</h2>
          <div className="row gap">
            <button className="btn btn-play" onClick={runScript} disabled={running}>
              {running ? "⏳ Running…" : "▶ Run"}
            </button>
          </div>
        </div>
        <textarea
          className="script-editor"
          value={script}
          onChange={(e) => setScript(e.target.value)}
          spellCheck={false}
          rows={20}
        />
      </section>

      <section className="card">
        <div className="card-header">
          <h2>Save as Macro</h2>
          <div className="row gap">
            <input
              className="input"
              value={macroName}
              onChange={(e) => setMacroName(e.target.value)}
              placeholder="Macro name"
            />
            <button className="btn btn-secondary" onClick={saveScriptMacro}>
              💾 Save
            </button>
          </div>
        </div>
        <p className="muted">Saves the script as a playable macro in your macro list.</p>
      </section>

      {output.length > 0 && (
        <section className="card">
          <h2>Output</h2>
          <div className="script-output">
            {output.map((line, i) => (
              <div key={i} className={`output-line ${line.startsWith("ERROR") ? "error" : ""}`}>
                {line}
              </div>
            ))}
          </div>
        </section>
      )}

      {status && <div className="status-bar">{status}</div>}
    </div>
  );
}
