import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  listMarketplace,
  downloadMacro,
  uploadMacro,
  recordDownload,
  type MarketplaceListing,
} from "../lib/supabase";
import type { User } from "@supabase/supabase-js";

interface MacroData { name: string; events: unknown[]; duration_ms: number; tags: string[]; description: string }

interface Props {
  user: User | null;
  onRequestAuth: () => void;
}

const POPULAR_TAGS = ["gaming", "productivity", "web", "forms", "media", "utility"];

function fmtCents(c: number) {
  return c === 0 ? "Free" : `$${(c / 100).toFixed(2)}`;
}

export default function MarketplaceTab({ user, onRequestAuth }: Props) {
  const [listings, setListings] = useState<MarketplaceListing[]>([]);
  const [search, setSearch] = useState("");
  const [activeTag, setActiveTag] = useState<string | undefined>();
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  // Upload panel
  const [showUpload, setShowUpload] = useState(false);
  const [macros, setMacros] = useState<MacroData[]>([]);
  const [uploadMacroName, setUploadMacroName] = useState("");
  const [uploadTitle, setUploadTitle] = useState("");
  const [uploadDesc, setUploadDesc] = useState("");
  const [uploadTags, setUploadTags] = useState("");
  const [uploadPrice, setUploadPrice] = useState("0");
  const [uploading, setUploading] = useState(false);

  const fetchListings = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listMarketplace(search || undefined, activeTag);
      setListings(data);
    } catch (e) {
      setStatus(`Failed to load: ${e}`);
    }
    setLoading(false);
  }, [search, activeTag]);

  useEffect(() => {
    fetchListings();
  }, [fetchListings]);

  useEffect(() => {
    invoke<MacroData[]>("list_macros").then(setMacros).catch(() => {});
  }, [showUpload]);

  async function handleDownload(listing: MarketplaceListing) {
    if (!user) return onRequestAuth();
    setStatus(`Downloading "${listing.name}"…`);
    try {
      const json = await downloadMacro(listing);
      const parsed = JSON.parse(json);
      await invoke("save_macro", { macroData: parsed });
      await recordDownload(listing.id);
      setStatus(`"${listing.name}" added to your macros.`);
    } catch (e) {
      setStatus(`Download failed: ${e}`);
    }
  }

  async function handleUpload() {
    if (!user) return onRequestAuth();
    if (!uploadMacroName || !uploadTitle) return setStatus("Choose a macro and enter a title.");
    setUploading(true);
    setStatus("Uploading…");
    try {
      const macros = await invoke<MacroData[]>("list_macros");
      const m = macros.find((x) => x.name === uploadMacroName);
      if (!m) throw new Error("Macro not found");
      const tags = uploadTags.split(",").map((t) => t.trim()).filter(Boolean);
      const priceCents = Math.round(parseFloat(uploadPrice) * 100) || 0;
      await uploadMacro(JSON.stringify(m), uploadTitle, uploadDesc, tags, priceCents, user.id);
      setStatus("Macro published to marketplace!");
      setShowUpload(false);
      fetchListings();
    } catch (e) {
      setStatus(`Upload failed: ${e}`);
    }
    setUploading(false);
  }

  return (
    <div className="tab-layout single-col">
      {/* Search + filter bar */}
      <section className="card">
        <div className="marketplace-toolbar">
          <input
            className="input search-input"
            placeholder="Search macros…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && fetchListings()}
          />
          <button className="btn btn-play" onClick={fetchListings}>Search</button>
          {user ? (
            <button className="btn btn-secondary" onClick={() => setShowUpload(!showUpload)}>
              {showUpload ? "Cancel" : "↑ Publish"}
            </button>
          ) : (
            <button className="btn btn-secondary" onClick={onRequestAuth}>Sign In</button>
          )}
        </div>
        <div className="tag-filter-row">
          <button className={`tag-chip ${!activeTag ? "active" : ""}`} onClick={() => setActiveTag(undefined)}>All</button>
          {POPULAR_TAGS.map((t) => (
            <button key={t} className={`tag-chip ${activeTag === t ? "active" : ""}`} onClick={() => setActiveTag(t === activeTag ? undefined : t)}>
              {t}
            </button>
          ))}
        </div>
      </section>

      {/* Upload panel */}
      {showUpload && (
        <section className="card">
          <h2>Publish a Macro</h2>
          <div className="form-grid">
            <label className="option-row">
              <span>Macro</span>
              <select className="input" value={uploadMacroName} onChange={(e) => setUploadMacroName(e.target.value)}>
                <option value="">— Select —</option>
                {macros.map((m) => <option key={m.name}>{m.name}</option>)}
              </select>
            </label>
            <label className="option-row">
              <span>Title</span>
              <input className="input" value={uploadTitle} onChange={(e) => setUploadTitle(e.target.value)} placeholder="Public display name" />
            </label>
            <label className="option-row">
              <span>Description</span>
              <input className="input" value={uploadDesc} onChange={(e) => setUploadDesc(e.target.value)} placeholder="What does it do?" />
            </label>
            <label className="option-row">
              <span>Tags</span>
              <input className="input" value={uploadTags} onChange={(e) => setUploadTags(e.target.value)} placeholder="gaming, productivity, …" />
            </label>
            <label className="option-row">
              <span>Price (USD)</span>
              <input className="input input-sm" type="number" min="0" step="0.99" value={uploadPrice} onChange={(e) => setUploadPrice(e.target.value)} />
            </label>
          </div>
          <button className="btn btn-play" onClick={handleUpload} disabled={uploading}>
            {uploading ? "Publishing…" : "Publish"}
          </button>
        </section>
      )}

      {/* Listings grid */}
      <section className="card">
        <h2>Marketplace {loading && <span className="muted">Loading…</span>}</h2>
        {listings.length === 0 && !loading && <p className="empty-hint">No macros found.</p>}
        <div className="marketplace-grid">
          {listings.map((l) => (
            <div key={l.id} className="listing-card">
              <div className="listing-header">
                <span className="listing-name">{l.name}</span>
                <span className="listing-price">{fmtCents(l.price_cents)}</span>
              </div>
              <p className="listing-desc">{l.description}</p>
              <div className="listing-meta">
                {l.tags.map((t) => <span key={t} className="tag-chip sm">{t}</span>)}
              </div>
              <div className="listing-footer">
                <span className="muted">↓ {l.downloads} · ★ {l.rating.toFixed(1)}</span>
                <button className="btn btn-play btn-small" onClick={() => handleDownload(l)}>
                  {l.price_cents === 0 ? "Download" : "Buy"}
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>

      {status && <div className="status-bar">{status}</div>}
    </div>
  );
}
