import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LicenseInfo {
  tier: string;
  valid: boolean;
  features: string[];
  expires_ms: number | null;
  cached_at_ms: number;
}

interface Props {
  onClose: () => void;
}

export default function LicenseModal({ onClose }: Props) {
  const [info, setInfo] = useState<LicenseInfo | null>(null);
  const [machineId, setMachineId] = useState("");
  const [key, setKey] = useState("");
  const [supabaseUrl] = useState(import.meta.env.VITE_SUPABASE_URL ?? "");
  const [anonKey] = useState(import.meta.env.VITE_SUPABASE_ANON_KEY ?? "");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  useEffect(() => {
    load();
  }, []);

  async function load() {
    const [li, mid] = await Promise.all([
      invoke<LicenseInfo>("get_license_info"),
      invoke<string>("get_machine_id_cmd"),
    ]);
    setInfo(li);
    setMachineId(mid);
  }

  async function activate() {
    if (!key.trim()) return setStatus("Enter a license key.");
    setLoading(true);
    setStatus("");
    try {
      const li = await invoke<LicenseInfo>("activate_license", {
        key: key.trim(),
        supabaseUrl,
        anonKey,
      });
      setInfo(li);
      setStatus(li.valid ? "License activated!" : "License invalid or expired.");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
    setLoading(false);
  }

  async function refresh() {
    setLoading(true);
    try {
      await invoke("refresh_license", { supabaseUrl, anonKey });
      await load();
      setStatus("License refreshed.");
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
    setLoading(false);
  }

  async function deactivate() {
    await invoke("deactivate_license");
    await load();
    setStatus("License deactivated.");
  }

  const tierBadgeClass = info?.tier === "lifetime" ? "tier-lifetime"
    : info?.tier === "pro" ? "tier-pro"
    : "tier-free";

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>License</h2>
          <button className="icon-btn" onClick={onClose}>✕</button>
        </div>

        {info && (
          <div className="license-status">
            <span className={`tier-badge ${tierBadgeClass}`}>{info.tier.toUpperCase()}</span>
            {info.valid
              ? <span className="badge-green">Active</span>
              : <span className="badge-red">Inactive</span>}
            {info.expires_ms && (
              <span className="muted">Expires: {new Date(info.expires_ms).toLocaleDateString()}</span>
            )}
          </div>
        )}

        {info && info.features.length > 0 && (
          <div className="feature-pills">
            {info.features.map((f) => <span key={f} className="feature-pill">{f}</span>)}
          </div>
        )}

        <div className="form-col">
          <label className="option-row">
            <span>License Key</span>
            <input
              className="input"
              placeholder="XXXX-XXXX-XXXX-XXXX"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
          </label>
          <div className="row gap">
            <button className="btn btn-play" onClick={activate} disabled={loading}>Activate</button>
            {info?.valid && <button className="btn btn-secondary" onClick={refresh} disabled={loading}>Refresh</button>}
            {info?.valid && <button className="btn btn-danger" onClick={deactivate}>Deactivate</button>}
          </div>
        </div>

        <div className="machine-id-block">
          <span className="muted">Machine ID (share with support):</span>
          <code className="machine-id">{machineId}</code>
        </div>

        {status && <div className="status-bar">{status}</div>}
      </div>
    </div>
  );
}
