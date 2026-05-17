import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onAuthChange, signOut, getUser } from "./lib/supabase";
import type { User } from "@supabase/supabase-js";
import MacrosTab, { type MacroData } from "./components/MacrosTab";
import AutoClickerTab, { type AutoClickerConfig } from "./components/AutoClickerTab";
import ProfilesTab, { type Profile } from "./components/ProfilesTab";
import ScriptTab from "./components/ScriptTab";
import SchedulerTab from "./components/SchedulerTab";
import MarketplaceTab from "./components/MarketplaceTab";
import AuthModal from "./components/AuthModal";
import LicenseModal from "./components/LicenseModal";
import "./App.css";

type Tab = "macros" | "autoclicker" | "profiles" | "script" | "scheduler" | "marketplace";

export default function App() {
  const [tab, setTab] = useState<Tab>("macros");
  const [macros, setMacros] = useState<MacroData[]>([]);
  const [recording, setRecording] = useState(false);
  const [paused, setPaused] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [autoclicking, setAutoclicking] = useState(false);
  const [activeProfile, setActiveProfile] = useState<Profile | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [showAuth, setShowAuth] = useState(false);
  const [showLicense, setShowLicense] = useState(false);

  const [speed, setSpeed] = useState(1.0);
  const [loops, setLoops] = useState(1);
  const [acConfig, setAcConfig] = useState<Partial<AutoClickerConfig>>({});

  useEffect(() => {
    loadMacros();
    loadActiveProfile();
    getUser().then(setUser).catch(() => {});

    const unsub = onAuthChange(setUser);

    const poll = setInterval(async () => {
      const s = await invoke<{ recording: boolean; paused: boolean; playing: boolean; autoclicking: boolean }>("get_status");
      setRecording(s.recording);
      setPaused(s.paused);
      setPlaying(s.playing);
      setAutoclicking(s.autoclicking);
    }, 500);

    return () => {
      clearInterval(poll);
      unsub.data.subscription.unsubscribe();
    };
  }, []);

  async function loadMacros() {
    const list = await invoke<MacroData[]>("list_macros");
    setMacros(list);
  }

  async function loadActiveProfile() {
    const p = await invoke<Profile>("get_active_profile");
    applyProfileState(p);
  }

  function applyProfileState(p: Profile) {
    setActiveProfile(p);
    setSpeed(p.playback_speed);
    setLoops(p.playback_loops);
    setAcConfig(p.autoclicker);
  }

  async function handleSignOut() {
    await signOut();
    setUser(null);
  }

  const statusDots: { label: string; active: boolean }[] = [
    { label: "REC", active: recording },
    { label: "PLAY", active: playing },
    { label: "AC", active: autoclicking },
  ];

  const tabLabels: Record<Tab, string> = {
    macros: "Macros",
    autoclicker: "Auto-Clicker",
    profiles: "Profiles",
    script: "Script",
    scheduler: "Scheduler",
    marketplace: "Marketplace",
  };

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-left">
          <h1>Macro Recorder</h1>
          <span className="version">v0.3.0</span>
        </div>
        <nav className="tabs">
          {(Object.keys(tabLabels) as Tab[]).map((t) => (
            <button key={t} className={`tab-btn ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
              {tabLabels[t]}
            </button>
          ))}
        </nav>
        <div className="header-right">
          {statusDots.map(({ label, active }) => (
            <span key={label} className={`status-chip ${active ? "on" : ""}`}>{label}</span>
          ))}
          {activeProfile && (
            <span className="profile-badge" onClick={() => setTab("profiles")}>
              {activeProfile.name}
            </span>
          )}
          {user ? (
            <span className="user-chip" title={user.email} onClick={handleSignOut}>
              {user.email?.split("@")[0]}
            </span>
          ) : (
            <button className="btn btn-small" onClick={() => setShowAuth(true)}>Sign In</button>
          )}
          <button className="icon-btn" title="License" onClick={() => setShowLicense(true)}>🔑</button>
        </div>
      </header>

      <div className="tab-content">
        {tab === "macros" && (
          <MacrosTab
            macros={macros}
            recording={recording}
            playing={playing}
            paused={paused}
            speed={speed}
            loops={loops}
            onMacrosChange={loadMacros}
            onSpeedChange={setSpeed}
            onLoopsChange={setLoops}
          />
        )}
        {tab === "autoclicker" && (
          <AutoClickerTab
            initialConfig={acConfig}
            onConfigChange={(cfg) => setAcConfig(cfg)}
          />
        )}
        {tab === "profiles" && (
          <ProfilesTab
            activeProfileName={activeProfile?.name ?? "Default"}
            onApply={applyProfileState}
          />
        )}
        {tab === "script" && <ScriptTab />}
        {tab === "scheduler" && <SchedulerTab />}
        {tab === "marketplace" && (
          <MarketplaceTab
            user={user}
            onRequestAuth={() => setShowAuth(true)}
          />
        )}
      </div>

      {showAuth && (
        <AuthModal
          onAuth={(u) => { setUser(u); setShowAuth(false); }}
          onClose={() => setShowAuth(false)}
        />
      )}
      {showLicense && <LicenseModal onClose={() => setShowLicense(false)} />}
    </div>
  );
}
