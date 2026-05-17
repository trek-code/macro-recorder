import { useState } from "react";
import { signIn, signUp } from "../lib/supabase";
import type { User } from "@supabase/supabase-js";

interface Props {
  onAuth: (user: User) => void;
  onClose: () => void;
}

export default function AuthModal({ onAuth, onClose }: Props) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function submit() {
    setError("");
    setLoading(true);
    try {
      const data = mode === "login"
        ? await signIn(email, password)
        : await signUp(email, password);
      if (data.user) {
        onAuth(data.user);
      } else {
        setError(mode === "register" ? "Check your email to confirm your account." : "Login failed.");
      }
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{mode === "login" ? "Sign In" : "Create Account"}</h2>
          <button className="icon-btn" onClick={onClose}>✕</button>
        </div>
        <div className="form-col">
          <input
            className="input"
            type="email"
            placeholder="Email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <input
            className="input"
            type="password"
            placeholder="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
          {error && <p className="error-text">{error}</p>}
          <button className="btn btn-play" onClick={submit} disabled={loading}>
            {loading ? "…" : mode === "login" ? "Sign In" : "Create Account"}
          </button>
          <button className="btn-link" onClick={() => setMode(mode === "login" ? "register" : "login")}>
            {mode === "login" ? "Don't have an account? Register" : "Already have an account? Sign in"}
          </button>
        </div>
      </div>
    </div>
  );
}
