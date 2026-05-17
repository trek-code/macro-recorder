import { useState, useEffect } from "react";

const CURRENT_VERSION = "0.3.0";
const CHECK_URL = "https://trek-code.github.io/macro-recorder/latest.json";

interface LatestJson {
  version: string;
  notes?: string;
  url: string;
}

function semverGt(a: string, b: string): boolean {
  const parse = (v: string) => v.split(".").map(Number);
  const [a0, a1, a2] = parse(a);
  const [b0, b1, b2] = parse(b);
  if (a0 !== b0) return a0 > b0;
  if (a1 !== b1) return a1 > b1;
  return a2 > b2;
}

export default function UpdateBanner() {
  const [update, setUpdate] = useState<LatestJson | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    fetch(CHECK_URL)
      .then((r) => r.json())
      .then((data: LatestJson) => {
        if (semverGt(data.version, CURRENT_VERSION)) setUpdate(data);
      })
      .catch(() => {});
  }, []);

  if (!update || dismissed) return null;

  return (
    <div className="update-banner">
      <span>
        <strong>v{update.version}</strong> is available
        {update.notes ? ` — ${update.notes}` : ""}
      </span>
      <div className="update-banner-actions">
        <a className="update-link" href={update.url} target="_blank" rel="noreferrer">
          Download
        </a>
        <button className="update-dismiss" onClick={() => setDismissed(true)}>✕</button>
      </div>
    </div>
  );
}
