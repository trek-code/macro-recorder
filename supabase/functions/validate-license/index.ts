import { serve } from "https://deno.land/std@0.168.0/http/server.ts";
import { createClient } from "https://esm.sh/@supabase/supabase-js@2";

const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
};

serve(async (req) => {
  if (req.method === "OPTIONS") {
    return new Response("ok", { headers: corsHeaders });
  }

  try {
    const { key, machine_id } = await req.json();
    if (!key || !machine_id) {
      return new Response(JSON.stringify({ error: "Missing key or machine_id" }), {
        status: 400,
        headers: { ...corsHeaders, "Content-Type": "application/json" },
      });
    }

    const supabase = createClient(
      Deno.env.get("SUPABASE_URL")!,
      Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!
    );

    const { data: license, error } = await supabase
      .from("licenses")
      .select("*")
      .eq("key", key)
      .single();

    if (error || !license) {
      return new Response(JSON.stringify({ valid: false, reason: "not_found" }), {
        headers: { ...corsHeaders, "Content-Type": "application/json" },
      });
    }

    if (license.revoked) {
      return new Response(JSON.stringify({ valid: false, reason: "revoked" }), {
        headers: { ...corsHeaders, "Content-Type": "application/json" },
      });
    }

    if (license.expires_at && new Date(license.expires_at) < new Date()) {
      return new Response(JSON.stringify({ valid: false, reason: "expired" }), {
        headers: { ...corsHeaders, "Content-Type": "application/json" },
      });
    }

    // Bind machine if not yet bound
    if (!license.machine_id) {
      await supabase
        .from("licenses")
        .update({ machine_id, activations: 1 })
        .eq("key", key);
    } else if (license.machine_id !== machine_id) {
      // Already bound to a different machine
      return new Response(
        JSON.stringify({ valid: false, reason: "machine_mismatch" }),
        { headers: { ...corsHeaders, "Content-Type": "application/json" } }
      );
    }

    return new Response(
      JSON.stringify({
        valid: true,
        tier: license.tier,
        features: license.features,
        expires_ms: license.expires_at ? new Date(license.expires_at).getTime() : null,
      }),
      { headers: { ...corsHeaders, "Content-Type": "application/json" } }
    );
  } catch (e) {
    return new Response(JSON.stringify({ error: String(e) }), {
      status: 500,
      headers: { ...corsHeaders, "Content-Type": "application/json" },
    });
  }
});
