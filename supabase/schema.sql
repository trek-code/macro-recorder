-- ── Extensions ───────────────────────────────────────────────────────────────
create extension if not exists "pgcrypto";

-- ── marketplace_listings ──────────────────────────────────────────────────────
create table if not exists marketplace_listings (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references auth.users(id) on delete cascade,
  name          text not null,
  description   text not null default '',
  price_cents   integer not null default 0 check (price_cents >= 0),
  tags          text[] not null default '{}',
  downloads     integer not null default 0,
  rating        numeric(3,1) not null default 0.0,
  macro_path    text not null,          -- Supabase Storage path: {user_id}/{timestamp}_{name}.json
  preview_events integer not null default 0,
  published     boolean not null default true,
  created_at    timestamptz not null default now()
);

alter table marketplace_listings enable row level security;

-- Anyone can read published listings
create policy "Public read" on marketplace_listings
  for select using (published = true);

-- Authenticated users can insert their own
create policy "Owner insert" on marketplace_listings
  for insert with check (auth.uid() = user_id);

-- Owners can update/delete their own
create policy "Owner update" on marketplace_listings
  for update using (auth.uid() = user_id);

create policy "Owner delete" on marketplace_listings
  for delete using (auth.uid() = user_id);

-- ── increment_downloads RPC ───────────────────────────────────────────────────
create or replace function increment_downloads(listing_id uuid)
returns void language sql security definer as $$
  update marketplace_listings
  set downloads = downloads + 1
  where id = listing_id;
$$;

-- ── licenses ──────────────────────────────────────────────────────────────────
create table if not exists licenses (
  id            uuid primary key default gen_random_uuid(),
  key           text not null unique,
  tier          text not null default 'pro' check (tier in ('pro', 'lifetime')),
  features      text[] not null default '{}',
  machine_id    text,                   -- bound machine (null = unbound)
  max_machines  integer not null default 1,
  activations   integer not null default 0,
  expires_at    timestamptz,            -- null = never
  created_at    timestamptz not null default now(),
  revoked       boolean not null default false
);

-- No direct client access — only edge functions touch this table
alter table licenses enable row level security;

-- ── Storage: macros bucket ────────────────────────────────────────────────────
-- Run these in the Supabase dashboard Storage section or via CLI:
--
--   supabase storage buckets create macros --public false
--
-- Then add policies so authenticated users can upload to their own folder,
-- and anyone can download (for free macros) — adjust as needed for paid access.

-- ── Edge function: validate-license ──────────────────────────────────────────
-- Deploy with: supabase functions deploy validate-license
-- File: supabase/functions/validate-license/index.ts  (see below)
