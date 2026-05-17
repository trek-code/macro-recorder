#!/usr/bin/env node
/**
 * License key generator for Macro Recorder
 *
 * Usage:
 *   node scripts/generate-license.mjs [options]
 *
 * Options:
 *   --tier pro|lifetime          (default: pro)
 *   --features f1,f2,...         (default: marketplace,scheduler,scripting)
 *   --expires YYYY-MM-DD         (default: none — never expires)
 *   --count N                    (default: 1 — generate N keys at once)
 *   --dry-run                    (print keys but don't insert into DB)
 *
 * Requires env vars:
 *   SUPABASE_URL
 *   SUPABASE_SERVICE_ROLE_KEY    (NOT the anon key — this needs admin access)
 *
 * Example:
 *   SUPABASE_URL=https://xxx.supabase.co \
 *   SUPABASE_SERVICE_ROLE_KEY=eyJ... \
 *   node scripts/generate-license.mjs --tier lifetime --count 5
 */

import { createClient } from "@supabase/supabase-js";
import { randomBytes } from "crypto";

// ── Parse args ────────────────────────────────────────────────────────────────

const args = process.argv.slice(2);

function getArg(flag, fallback) {
  const i = args.indexOf(flag);
  return i !== -1 ? args[i + 1] : fallback;
}

const tier       = getArg("--tier", "pro");
const features   = getArg("--features", "marketplace,scheduler,scripting").split(",").map((s) => s.trim());
const expiresRaw = getArg("--expires", null);
const count      = parseInt(getArg("--count", "1"), 10);
const dryRun     = args.includes("--dry-run");

if (!["pro", "lifetime"].includes(tier)) {
  console.error("--tier must be pro or lifetime");
  process.exit(1);
}

const expiresAt = expiresRaw ? new Date(expiresRaw).toISOString() : null;

// ── Key generation ────────────────────────────────────────────────────────────

function generateKey() {
  // Format: XXXX-XXXX-XXXX-XXXX (hex, uppercase)
  const bytes = randomBytes(8);
  const hex = bytes.toString("hex").toUpperCase();
  return [
    hex.slice(0, 4),
    hex.slice(4, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
  ].join("-");
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  const supabaseUrl = process.env.SUPABASE_URL;
  const serviceKey  = process.env.SUPABASE_SERVICE_ROLE_KEY;

  if (!supabaseUrl || !serviceKey) {
    console.error("Set SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY env vars.");
    if (!dryRun) process.exit(1);
  }

  const keys = Array.from({ length: count }, generateKey);

  if (dryRun) {
    console.log("\n=== DRY RUN — keys NOT saved ===\n");
    keys.forEach((k) => console.log(k));
    return;
  }

  const supabase = createClient(supabaseUrl, serviceKey);

  const rows = keys.map((key) => ({
    key,
    tier,
    features,
    expires_at: expiresAt,
  }));

  const { data, error } = await supabase
    .from("licenses")
    .insert(rows)
    .select("key, tier, features, expires_at");

  if (error) {
    console.error("Insert failed:", error.message);
    process.exit(1);
  }

  console.log(`\n✓ Generated ${data.length} license key(s) [${tier.toUpperCase()}]\n`);
  data.forEach((row) => {
    const exp = row.expires_at ? `  expires ${row.expires_at.slice(0, 10)}` : "  never expires";
    console.log(`  ${row.key}${exp}`);
  });
  console.log("");
}

main().catch((e) => { console.error(e); process.exit(1); });
