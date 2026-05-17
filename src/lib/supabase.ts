import { createClient, SupabaseClient, User } from "@supabase/supabase-js";

// Replace these with your Supabase project values.
// Create a .env file or set them directly here for development.
const SUPABASE_URL = import.meta.env.VITE_SUPABASE_URL ?? "";
const SUPABASE_ANON_KEY = import.meta.env.VITE_SUPABASE_ANON_KEY ?? "";

export const supabase: SupabaseClient = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);

export { SUPABASE_URL, SUPABASE_ANON_KEY };

// ── Auth helpers ──────────────────────────────────────────────────────────────

export async function signIn(email: string, password: string) {
  const { data, error } = await supabase.auth.signInWithPassword({ email, password });
  if (error) throw error;
  return data;
}

export async function signUp(email: string, password: string) {
  const { data, error } = await supabase.auth.signUp({ email, password });
  if (error) throw error;
  return data;
}

export async function signOut() {
  await supabase.auth.signOut();
}

export async function getUser(): Promise<User | null> {
  const { data } = await supabase.auth.getUser();
  return data.user;
}

export function onAuthChange(callback: (user: User | null) => void) {
  return supabase.auth.onAuthStateChange((_event, session) => {
    callback(session?.user ?? null);
  });
}

// ── Marketplace helpers ───────────────────────────────────────────────────────

export interface MarketplaceListing {
  id: string;
  user_id: string;
  name: string;
  description: string;
  price_cents: number;  // 0 = free
  tags: string[];
  downloads: number;
  rating: number;
  macro_path: string;   // Supabase Storage path
  preview_events: number;
  created_at: string;
  author_email?: string;
}

export async function listMarketplace(search?: string, tag?: string): Promise<MarketplaceListing[]> {
  let query = supabase
    .from("marketplace_listings")
    .select("*")
    .eq("published", true)
    .order("downloads", { ascending: false });

  if (search) {
    query = query.ilike("name", `%${search}%`);
  }
  if (tag) {
    query = query.contains("tags", [tag]);
  }

  const { data, error } = await query.limit(50);
  if (error) throw error;
  return data ?? [];
}

export async function downloadMacro(listing: MarketplaceListing): Promise<string> {
  const { data, error } = await supabase.storage
    .from("macros")
    .download(listing.macro_path);
  if (error) throw error;
  return await data.text();
}

export async function uploadMacro(
  macroJson: string,
  name: string,
  description: string,
  tags: string[],
  priceCents: number,
  userId: string
) {
  const path = `${userId}/${Date.now()}_${name.replace(/[^a-z0-9]/gi, "_")}.json`;
  const blob = new Blob([macroJson], { type: "application/json" });

  const { error: uploadError } = await supabase.storage
    .from("macros")
    .upload(path, blob);
  if (uploadError) throw uploadError;

  const { data, error } = await supabase.from("marketplace_listings").insert({
    name,
    description,
    tags,
    price_cents: priceCents,
    macro_path: path,
    preview_events: JSON.parse(macroJson).events?.length ?? 0,
    user_id: userId,
    published: true,
  }).select().single();

  if (error) throw error;
  return data;
}

export async function recordDownload(listingId: string) {
  await supabase.rpc("increment_downloads", { listing_id: listingId });
}
