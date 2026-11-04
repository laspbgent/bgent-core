//! JSON serialisation for `Report`, hand-rolled so the crate stays dependency-free.
//! The schema is a contract with the Rules Agent's reader and the study's data
//! collection, so keys are stable and snake_case.

use crate::harness::{Playout, Report, Severity};

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// JSON has no NaN or Infinity. Rather than emit `null` and lose the fact that a
/// score broke, non-finite values are recorded as strings.
fn score(x: f32) -> String {
    if x.is_finite() {
        format!("{}", x)
    } else {
        esc(&format!("{}", x))
    }
}

fn scores(v: &[f32]) -> String {
    let parts: Vec<String> = v.iter().map(|x| score(*x)).collect();
    format!("[{}]", parts.join(","))
}

fn nums(v: &[usize]) -> String {
    let parts: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn sev(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn playout_json(p: &Playout) -> String {
    let mut o = String::from("{");
    o.push_str(&format!("\"label\":{},", esc(&p.label)));
    o.push_str(&format!("\"plies\":{},", p.plies));
    o.push_str(&format!("\"reached_terminal\":{},", p.reached_terminal));
    o.push_str(&format!("\"hit_ply_cap\":{},", p.hit_ply_cap));
    o.push_str(&format!("\"final_scores\":{},", scores(&p.final_scores)));
    o.push_str("\"winner\":");
    match p.winner {
        Some(w) => o.push_str(&w.to_string()),
        None => o.push_str("null"),
    }
    o.push_str(",\"moves\":[");
    let moves: Vec<String> = p
        .moves
        .iter()
        .map(|(who, desc)| format!("{{\"player\":{},\"move\":{}}}", who, esc(desc)))
        .collect();
    o.push_str(&moves.join(","));
    o.push_str("]}");
    o
}

impl Report {
    pub fn to_json(&self, seed: u64) -> String {
        let mut o = String::new();
        o.push_str("{\n");
        o.push_str("  \"schema\": \"bgent.report/1\",\n");
        o.push_str(&format!("  \"seed\": {},\n", seed));
        o.push_str(&format!("  \"ok\": {},\n", self.ok()));
        o.push_str(&format!(
            "  \"counts\": {{\"errors\": {}, \"warnings\": {}}},\n",
            self.errors(),
            self.warnings()
        ));
        o.push_str(&format!(
            "  \"game\": {{\"num_players\": {}, \"base_moves\": {}, \"player_codes\": {}}},\n",
            self.num_players,
            self.base_move_count,
            nums(&self.player_codes_seen)
        ));
        o.push_str(&format!(
            "  \"crawl\": {{\"states_visited\": {}, \"edges_examined\": {}, \
             \"terminal_states\": {}, \"truncated\": {}, \"reachable_depth\": {}, \
             \"distinct_scores\": {}}},\n",
            self.states_visited,
            self.edges_examined,
            self.terminal_states_found,
            self.crawl_truncated,
            self.reachable_depth,
            self.distinct_scores
        ));
        let stuck: Vec<String> = self
            .playouts
            .iter()
            .filter(|p| !p.reached_terminal)
            .map(|p| esc(&p.label))
            .collect();
        o.push_str(&format!(
            "  \"playouts\": {{\"count\": {}, \"not_terminal\": [{}]}},\n",
            self.playouts.len(),
            stuck.join(",")
        ));

        o.push_str("  \"findings\": [");
        if self.findings.is_empty() {
            o.push(']');
        } else {
            o.push('\n');
            for (i, f) in self.findings.iter().enumerate() {
                o.push_str(&format!(
                    "    {{\"severity\": {}, \"code\": {}, \"detail\": {}}}",
                    esc(sev(f.severity)),
                    esc(f.code),
                    esc(&f.detail)
                ));
                o.push_str(if i + 1 == self.findings.len() { "\n" } else { ",\n" });
            }
            o.push_str("  ]");
        }
        o.push_str(",\n");

        o.push_str("  \"traces\": [");
        if self.playouts.is_empty() {
            o.push(']');
        } else {
            o.push('\n');
            for (i, p) in self.playouts.iter().enumerate() {
                o.push_str("    ");
                o.push_str(&playout_json(p));
                o.push_str(if i + 1 == self.playouts.len() { "\n" } else { ",\n" });
            }
            o.push_str("  ]");
        }
        o.push_str("\n}\n");
        o
    }
}
