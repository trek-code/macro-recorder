import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LicenseInfo {
  tier: string;
  valid: boolean;
  features: string[];
  expires_ms: number | null;
  cached_at_ms: number;
}

interface Props {
  onUnlocked: () => void;
}

export default function LicenseGate({ onUnlocked }: Props) {
  const [key, setKey] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [machineId, setMachineId] = useState("");
  const [showMachineId, setShowMachineId] = useState(false);

  async function activate() {
    if (!key.trim()) return setError("Enter a license key.");
    setLoading(true);
    setError("");
    try {
      const info = await invoke<LicenseInfo>("activate_license", {
        key: key.trim(),
        supabaseUrl: import.meta.env.VITE_SUPABASE_URL ?? "",
        anonKey: import.meta.env.VITE_SUPABASE_ANON_KEY ?? "",
      });
      if (info.valid) {
        onUnlocked();
      } else {
        setError("License key is invalid or expired.");
      }
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  }

  async function loadMachineId() {
    const id = await invoke<string>("get_machine_id_cmd");
    setMachineId(id);
    setShowMachineId(true);
  }

  return (
    <div className="gate-overlay">
      <div className="gate-box">
        <div className="gate-logo">
          Macro<span>Recorder</span>
        </div>
        <p className="gate-tagline">Automate anything on your PC</p>

        <div className="gate-form">
          <input
            className="input gate-input"
            placeholder="XXXX-XXXX-XXXX-XXXX"
            value={key}
            onChange={(e) => setKey(e.target.value.toUpperCase())}
            onKeyDown={(e) => e.key === "Enter" && activate()}
            spellCheck={false}
            autoFocus
          />
          {error && <p className="gate-error">{error}</p>}
          <button className="btn btn-record gate-btn" onClick={activate} disabled={loading}>
            {loading ? "Verifying…" : "Activate License"}
          </button>
        </div>

        <div className="gate-footer">
          <button className="btn-link" onClick={loadMachineId}>
            Need help? Show machine ID
          </button>
          {showMachineId && (
            <code className="gate-machine-id">{machineId}</code>
          )}
        </div>
      </div>
    </div>
  );
}
