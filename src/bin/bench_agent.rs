//! Benchmark agent quality across different settings.
//! Run: cargo run --bin bench_agent
//!
//! Tests character creation conversation with different max_token limits
//! to find optimal settings for quality vs verbosity.

use polit::ai::memory::Exchange;
use polit::ai::native_format;
use polit::ai::{AiProvider, DmMode};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let hf_token = std::env::var("HF_TOKEN").ok();
    let model_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "google/gemma-4-E4B-it".to_string());

    eprintln!("Loading {}...", model_id);

    // Suppress stderr during load
    let mut provider = {
        use std::os::unix::io::AsRawFd;
        let devnull = std::fs::File::open("/dev/null").ok();
        let saved = unsafe { libc::dup(2) };
        if let Some(ref null) = devnull {
            unsafe { libc::dup2(null.as_raw_fd(), 2); }
        }
        let result = polit::ai::provider::CandleProvider::load(&model_id, hf_token.as_deref());
        if saved >= 0 {
            unsafe { libc::dup2(saved, 2); libc::close(saved); }
        }
        result
    }?;

    eprintln!("Ready.\n");

    let system = "You are the AI for POLIT, an American politics simulator.\n\
        Tone: Sharp, funny, vivid. Match the player's energy.\n\n\
        You are a sharp, funny creative partner. The player is telling you about a character \
        for a political RPG. Your job is to GET INTO IT. Riff on everything they say.\n\
        YES AND everything. Build on their ideas. Add vivid details they didn't think of.\n\
        When they tell you something concrete about their character, quietly save it \
        with lock_field in the background — but keep the conversation flowing naturally.\n\
        Valid fields: background, motivation, archetype, starting_office, party, traits, family, rival, secret, tone\n\n\
        Use the declared tools via tool_call tokens. \
        Use channel thought to reason privately before responding.";

    let tool_decl = native_format::tool_declarations(DmMode::CharacterCreation);

    // Test prompts
    let test_turns: Vec<(&str, &str)> = vec![
        ("greeting", "My character's name is Homer Simpson. Let's figure out who they are."),
        ("background", "He works at a nuclear power plant and is incredibly lazy but somehow keeps his job"),
        ("vague", "I dunno, he just goes with the flow"),
        ("creative", "He once saved the town by accidentally crashing a truck into a dam"),
    ];

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  POLIT Agent Benchmark                                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Run 4-turn conversation for each token limit
    for max_tokens in [256, 384, 512, 768] {
        println!("━━━ max_tokens = {} ━━━", max_tokens);

        let mut history: Vec<Exchange> = Vec::new();

        for (label, user_input) in &test_turns {
            let prompt = native_format::build_prompt(
                system,
                &tool_decl,
                &history,
                &format!("Player: {}", user_input),
            );

            let prompt_tokens = prompt.len() / 4; // rough estimate

            let start = Instant::now();
            // Use generate which goes through full parsing pipeline
            let result = provider.generate(&prompt, DmMode::CharacterCreation);
            let elapsed = start.elapsed();

            match result {
                Ok(resp) => {
                    let has_q = resp.narration.trim_end().ends_with('?');
                    let tools: Vec<String> = resp.tool_calls.iter().map(|t| format!("{:?}", t)).collect();

                    let narr_short = if resp.narration.len() > 100 {
                        format!("{}...", &resp.narration[..97])
                    } else {
                        resp.narration.clone()
                    };

                    println!(
                        "  [{:10}] {:>5}ms | prompt:~{}tok | narr:{:>3}ch | tools:{} | q?:{} ",
                        label,
                        elapsed.as_millis(),
                        prompt_tokens,
                        resp.narration.len(),
                        resp.tool_calls.len(),
                        if has_q { "Y" } else { "N" },
                    );
                    println!("    → {}", narr_short);
                    for t in &tools {
                        let t_short = if t.len() > 80 { format!("{}...", &t[..77]) } else { t.clone() };
                        println!("    ⚙ {}", t_short);
                    }

                    // Add to history
                    history.push(Exchange {
                        turn: history.len() as u32 + 1,
                        user_input: user_input.to_string(),
                        assistant_response: resp.narration,
                        tool_calls_summary: vec![],
                        timestamp_week: 1,
                    });
                }
                Err(e) => {
                    println!("  [{:10}] ERROR: {}", label, e);
                }
            }
        }
        println!();
    }

    Ok(())
}
