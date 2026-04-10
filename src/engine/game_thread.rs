use std::thread;

use super::channels::{GameChannels, UiCommand, UiMessage};
use super::{GamePhase, GameState};
use crate::ai::context::{GameContext, NpcContext};
use crate::ai::tools::ToolCall;
use crate::ai::DmMode;
use crate::systems::dice;

/// Run the game logic on a dedicated thread
pub fn spawn_game_thread(mut state: GameState, channels: GameChannels) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("polit-game".to_string())
        .spawn(move || {
            game_loop_main(&mut state, &channels);
        })
        .expect("Failed to spawn game thread")
}

/// After the demo finishes, run the normal interactive game loop
pub fn spawn_game_loop_after_demo(state: GameState, ch: &GameChannels) {
    interactive_loop(state, ch);
}

fn game_loop_main(state: &mut GameState, ch: &GameChannels) {
    ch.send(UiMessage::System(
        "Welcome to POLIT — The American Politics Simulator".into(),
    ));
    ch.send(UiMessage::System(format!(
        "AI Provider: {} │ Difficulty: {}",
        state.ai.provider_name(),
        state.config.difficulty.description
    )));
    ch.send(UiMessage::System(
        "Type /help for commands, or just start typing.".into(),
    ));
    send_status(state, ch);

    run_dawn(state, ch);

    interactive_loop_ref(state, ch);
}

fn interactive_loop(mut state: GameState, ch: &GameChannels) {
    interactive_loop_ref(&mut state, ch);
}

fn interactive_loop_ref(state: &mut GameState, ch: &GameChannels) {
    loop {
        match ch.try_recv() {
            Some(UiCommand::Quit) => {
                ch.send(UiMessage::Shutdown);
                break;
            }
            Some(UiCommand::EndTurn) => {
                // Leave any active conversation
                if state.active_npc.is_some() {
                    state.active_npc = None;
                    ch.send(UiMessage::System("You end the conversation.".into()));
                }
                run_dusk(state, ch);
                advance_week(state);
                send_status(state, ch);
                run_dawn(state, ch);
            }
            Some(UiCommand::PlayerInput(text)) => {
                handle_player_input(state, ch, &text);
            }
            Some(UiCommand::SlashCommand { cmd, args }) => {
                handle_slash_command(state, ch, &cmd, &args);
            }
            Some(UiCommand::SaveGame(name)) => match state.db.create_snapshot(&name) {
                Ok(path) => ch.send(UiMessage::Success(format!(
                    "Game saved: {}",
                    path.display()
                ))),
                Err(e) => ch.send(UiMessage::Warning(format!("Save failed: {}", e))),
            },
            Some(UiCommand::LoadGame(name)) => {
                ch.send(UiMessage::System(format!(
                    "Loading save '{}' — not yet implemented",
                    name
                )));
            }
            None => {
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

// ===== PHASE LOGIC =====

fn run_dawn(state: &mut GameState, ch: &GameChannels) {
    state.phase = GamePhase::Dawn;
    send_status(state, ch);

    // Seed the social graph on first week
    if state.week == 1 && state.social.character_count() == 0 {
        use crate::engine::components::{Relationship, RelationshipType};
        state.social.add_character("player", "Player", true);
        state
            .social
            .add_character("davis", "Councilwoman Davis", false);
        state
            .social
            .add_character("kowalski", "Chief Kowalski", false);
        state
            .social
            .add_character("martinez", "Sen. Martinez", false);
        state.social.add_character("chen", "Gov. Chen", false);
        state.social.add_character("kim", "Reporter Kim", false);

        state.social.set_relationship(
            "player",
            "davis",
            Relationship {
                trust: -15,
                respect: 30,
                fear: 0,
                loyalty: 10,
                debt: 0,
                knowledge: 20,
                leverage: 0,
                rel_type: RelationshipType::Rival,
                memories: vec![],
            },
        );
        state.social.set_relationship(
            "player",
            "kowalski",
            Relationship {
                trust: 30,
                respect: 50,
                fear: 0,
                loyalty: 25,
                debt: 0,
                knowledge: 15,
                leverage: 0,
                rel_type: RelationshipType::Neutral,
                memories: vec![],
            },
        );
        state.social.set_relationship(
            "player",
            "martinez",
            Relationship {
                trust: 40,
                respect: 35,
                fear: 0,
                loyalty: 20,
                debt: 1,
                knowledge: 10,
                leverage: 0,
                rel_type: RelationshipType::Ally,
                memories: vec![],
            },
        );
    }

    // Tick the economy simulation
    state.economy.tick();

    ch.send(UiMessage::PhaseHeader(format!(
        "Week {}, {} Begins",
        state.week, state.year
    )));

    // Use agent for morning briefing (with memory + tool execution)
    let ctx = build_context(state);
    let response = state.ai.run_turn(
        "Generate the morning briefing for this week.",
        &ctx,
        DmMode::Narrator,
        |tool| {
            // Tool execution during briefing — schedule events, update vars
            None
        },
    );
    ch.send(UiMessage::Narrate("■ MORNING BRIEFING".into()));
    ch.send(UiMessage::Narrate(response.narration));
    for tool in &response.executed_tools {
        process_tool_calls(state, ch, &[tool.clone()]);
    }

    ch.send(UiMessage::Narrate(format!(
        "\nYou have {} Action Points this week.",
        state.ap_current()
    )));

    state.phase = GamePhase::Action;
    send_status(state, ch);
}

fn run_dusk(state: &mut GameState, ch: &GameChannels) {
    state.phase = GamePhase::Dusk;
    send_status(state, ch);

    ch.send(UiMessage::System("Resolving consequences...".into()));

    // Autosave
    let save_name = format!("autosave_w{}_y{}", state.week, state.year);
    match state.db.create_snapshot(&save_name) {
        Ok(_) => ch.send(UiMessage::System("Autosaved.".into())),
        Err(e) => ch.send(UiMessage::Warning(format!("Autosave failed: {}", e))),
    }
}

fn advance_week(state: &mut GameState) {
    state.week += 1;
    if state.week > 52 {
        state.week = 1;
        state.year += 1;
    }
    // Reset AP from config + difficulty bonus
    let base_ap = state.config.balance.action_points.local;
    let bonus = state.config.difficulty.ap_bonus;
    state.reset_ap(base_ap + bonus);
}

// ===== INPUT HANDLING =====

fn handle_player_input(state: &mut GameState, ch: &GameChannels, text: &str) {
    if state.phase != GamePhase::Action {
        ch.send(UiMessage::Warning(
            "You can only act during the Action phase.".into(),
        ));
        return;
    }

    // If in a conversation, talk is free (no AP cost)
    if let Some(ref npc_name) = state.active_npc.clone() {
        ch.send(UiMessage::Narrate(format!("> {}", text)));

        let ctx = build_context_with_npc(state, npc_name);
        match state.ai.respond(text, &ctx, DmMode::Conversation) {
            Ok(response) => {
                ch.send(UiMessage::NpcDialogue {
                    name: npc_name.clone(),
                    text: response.narration,
                });
                process_tool_calls(state, ch, &response.tool_calls);
            }
            Err(e) => ch.send(UiMessage::Warning(format!("AI error: {}", e))),
        }
        send_status(state, ch);
        return;
    }

    // Free roam: costs 1 AP for general action
    ch.send(UiMessage::Narrate(format!("> {}", text)));

    let ctx = build_context(state);
    match state.ai.respond(text, &ctx, DmMode::DungeonMaster) {
        Ok(response) => {
            ch.send(UiMessage::Narrate(response.narration));
            process_tool_calls(state, ch, &response.tool_calls);
        }
        Err(e) => ch.send(UiMessage::Warning(format!("AI error: {}", e))),
    }

    state.spend_ap(1);
    if state.ap_current() <= 0 {
        ch.send(UiMessage::System(
            "You've used all your Action Points. Type /end to advance.".into(),
        ));
    }
    send_status(state, ch);
}

fn handle_slash_command(state: &mut GameState, ch: &GameChannels, cmd: &str, args: &[String]) {
    let costs = &state.config.balance.action_costs;

    match cmd {
        "meet" => {
            if args.is_empty() {
                ch.send(UiMessage::System("Usage: /meet <npc name>".into()));
                return;
            }
            let name = args.join(" ");
            let cost = costs.meet_in_person;
            if state.ap_current() < cost {
                ch.send(UiMessage::Warning(format!(
                    "Not enough AP. Meetings cost {} AP.",
                    cost
                )));
                return;
            }
            state.spend_ap(cost);
            state.active_npc = Some(name.clone());
            ch.send(UiMessage::System(format!(
                "You spend {} AP to meet with {}. (Type /leave to end meeting)",
                cost, name
            )));

            // AI generates NPC greeting
            let ctx = build_context_with_npc(state, &name);
            match state.ai.respond(
                &format!(
                    "The player has arrived to meet with {}. Generate the NPC's greeting.",
                    name
                ),
                &ctx,
                DmMode::Conversation,
            ) {
                Ok(response) => {
                    ch.send(UiMessage::NpcDialogue {
                        name: name.clone(),
                        text: response.narration,
                    });
                    process_tool_calls(state, ch, &response.tool_calls);
                }
                Err(e) => ch.send(UiMessage::Warning(format!("AI error: {}", e))),
            }
            send_status(state, ch);
        }
        "leave" => {
            if let Some(npc) = state.active_npc.take() {
                ch.send(UiMessage::System(format!(
                    "You end your meeting with {}.",
                    npc
                )));
            } else {
                ch.send(UiMessage::System("You're not in a conversation.".into()));
            }
            send_status(state, ch);
        }
        "call" => {
            if args.is_empty() {
                ch.send(UiMessage::System("Usage: /call <npc name>".into()));
                return;
            }
            let name = args.join(" ");
            let cost = costs.phone_call;
            if state.ap_current() < cost {
                ch.send(UiMessage::Warning(format!(
                    "Not enough AP. Phone calls cost {} AP.",
                    cost
                )));
                return;
            }
            state.spend_ap(cost);
            state.active_npc = Some(name.clone());
            ch.send(UiMessage::System(format!(
                "You spend {} AP to call {}. (Type /leave to hang up)",
                cost, name
            )));

            let ctx = build_context_with_npc(state, &name);
            match state.ai.respond(
                &format!("{} picks up the phone. Generate their greeting.", name),
                &ctx,
                DmMode::Conversation,
            ) {
                Ok(response) => {
                    ch.send(UiMessage::NpcDialogue {
                        name,
                        text: response.narration,
                    });
                    process_tool_calls(state, ch, &response.tool_calls);
                }
                Err(e) => ch.send(UiMessage::Warning(format!("AI error: {}", e))),
            }
            send_status(state, ch);
        }
        "speech" => {
            let cost = costs.speech;
            if state.ap_current() < cost {
                ch.send(UiMessage::Warning(format!(
                    "Not enough AP. Speeches cost {} AP.",
                    cost
                )));
                return;
            }
            state.spend_ap(cost);
            let topic = if args.is_empty() {
                "general".to_string()
            } else {
                args.join(" ")
            };
            ch.send(UiMessage::System(format!(
                "You spend {} AP to give a speech on: {}",
                cost, topic
            )));

            let ctx = build_context(state);
            match state.ai.respond(
                &format!("The player gives a public speech about '{}'. Narrate the scene and crowd reaction.", topic),
                &ctx,
                DmMode::DungeonMaster,
            ) {
                Ok(response) => {
                    ch.send(UiMessage::Narrate(response.narration));
                    process_tool_calls(state, ch, &response.tool_calls);
                }
                Err(e) => ch.send(UiMessage::Warning(format!("AI error: {}", e))),
            }
            send_status(state, ch);
        }
        "draft" => {
            ch.send(UiMessage::System(
                "Law drafting mode — type your proposal in plain English.".into(),
            ));
            ch.send(UiMessage::System(
                "(Full legislative pipeline coming in Phase 6)".into(),
            ));
        }
        "campaign" => {
            let cost = costs.campaign;
            if state.ap_current() < cost {
                ch.send(UiMessage::Warning(format!(
                    "Not enough AP. Campaigning costs {} AP.",
                    cost
                )));
                return;
            }
            state.spend_ap(cost);
            let district = if args.is_empty() {
                "your district".to_string()
            } else {
                args.join(" ")
            };
            ch.send(UiMessage::System(format!(
                "You spend {} AP campaigning in {}.",
                cost, district
            )));

            let ctx = build_context(state);
            match state.ai.respond(
                &format!(
                    "The player spends time campaigning in {}. Describe the reception.",
                    district
                ),
                &ctx,
                DmMode::Narrator,
            ) {
                Ok(response) => {
                    ch.send(UiMessage::Narrate(response.narration));
                    process_tool_calls(state, ch, &response.tool_calls);
                }
                Err(e) => ch.send(UiMessage::Warning(format!("AI error: {}", e))),
            }
            send_status(state, ch);
        }
        "save" => {
            let name = if args.is_empty() {
                format!("manual_w{}_y{}", state.week, state.year)
            } else {
                args.join("_")
            };
            match state.db.create_snapshot(&name) {
                Ok(path) => ch.send(UiMessage::Success(format!("Saved: {}", path.display()))),
                Err(e) => ch.send(UiMessage::Warning(format!("Save failed: {}", e))),
            }
        }
        "load" => {
            if args.is_empty() {
                ch.send(UiMessage::System("Usage: /load <save_name>".into()));
            } else {
                ch.send(UiMessage::System(format!(
                    "Loading '{}' — not yet implemented",
                    args.join(" ")
                )));
            }
        }
        _ => {
            ch.send(UiMessage::System(format!("Unknown command: /{}", cmd)));
        }
    }
}

// ===== AI CONTEXT =====

fn build_context(state: &GameState) -> GameContext {
    GameContext {
        week: state.week,
        year: state.year,
        phase: format!("{:?}", state.phase),
        player_name: "Player".into(),
        player_office: "City Council Member".into(),
        ap_current: state.ap_current(),
        ap_max: state.ap_max(),
        active_npcs: vec![],
        recent_events: vec![],
        economic_summary: state.economy.summary(),
        tone_instructions: "Realistic political drama. Grounded and human.".into(),
    }
}

fn build_context_with_npc(state: &GameState, npc_name: &str) -> GameContext {
    let mut ctx = build_context(state);

    // Try to find NPC in social graph by name match
    let npc_id = npc_name
        .to_lowercase()
        .replace(" ", "_")
        .replace("councilwoman_", "")
        .replace("chief_", "")
        .replace("sen._", "")
        .replace("gov._", "")
        .replace("reporter_", "");

    let (trust, respect, memories) =
        if let Some(rel) = state.social.get_relationship("player", &npc_id) {
            (
                rel.trust,
                rel.respect,
                rel.memories
                    .iter()
                    .map(|m| format!("Week {}: {}", m.week, m.description))
                    .collect(),
            )
        } else {
            (20, 40, vec![])
        };

    ctx.active_npcs.push(NpcContext {
        name: npc_name.to_string(),
        role: "Local political figure".into(),
        mood: if trust > 50 {
            "friendly"
        } else if trust < -20 {
            "hostile"
        } else {
            "neutral"
        }
        .into(),
        trust,
        respect,
        recent_memories: memories,
    });
    ctx
}

// ===== TOOL CALL PROCESSING =====

fn process_tool_calls(state: &mut GameState, ch: &GameChannels, calls: &[ToolCall]) {
    for call in calls {
        match call {
            ToolCall::Narrate { text } => {
                ch.send(UiMessage::Narrate(text.clone()));
            }
            ToolCall::ModifyRel { npc, field, delta } => {
                // Apply to social graph
                let npc_id = npc.to_lowercase().replace(" ", "_");
                state
                    .social
                    .modify_relationship("player", &npc_id, field, *delta);
                ch.send(UiMessage::System(format!(
                    "  [{}: {} {:+}]",
                    npc, field, delta
                )));
            }
            ToolCall::SetMood { npc, mood } => {
                ch.send(UiMessage::System(format!("  [{} mood → {}]", npc, mood)));
            }
            ToolCall::RollDice {
                skill,
                dc,
                modifier,
            } => {
                let result = dice::skill_check(skill, *modifier, *dc);
                let status = if result.critical_success {
                    "CRITICAL SUCCESS!"
                } else if result.critical_failure {
                    "CRITICAL FAILURE!"
                } else if result.success {
                    "Success"
                } else {
                    "Failure"
                };
                ch.send(UiMessage::DiceRoll(format!(
                    "🎲 {} check: {} + {} = {} vs DC {} → {}",
                    skill, result.natural_roll, result.modifiers, result.total, dc, status
                )));
            }
            ToolCall::GrantCard { card_id, reason } => {
                ch.send(UiMessage::Success(format!(
                    "📇 Card acquired: {} — {}",
                    card_id, reason
                )));
            }
            ToolCall::RevokeCard { card_id, reason } => {
                ch.send(UiMessage::Warning(format!(
                    "📇 Card lost: {} — {}",
                    card_id, reason
                )));
            }
            ToolCall::TriggerEvent {
                event_type,
                description,
            } => {
                ch.send(UiMessage::System(format!(
                    "⚡ Event: [{}] {}",
                    event_type, description
                )));
            }
            ToolCall::ScheduleEvent {
                event_type,
                description,
                weeks_ahead,
            } => {
                ch.send(UiMessage::System(format!(
                    "📅 Scheduled: [{}] {} (in {} weeks)",
                    event_type, description, weeks_ahead
                )));
            }
            ToolCall::ScoreAdjust { metric, delta } => {
                ch.send(UiMessage::System(format!("  [{}  {:+}]", metric, delta)));
            }
            ToolCall::SetDc { skill, dc, reason } => {
                ch.send(UiMessage::System(format!(
                    "  [DC set: {} DC {} — {}]",
                    skill, dc, reason
                )));
            }
            ToolCall::SpawnNpc { name, role, .. } => {
                ch.send(UiMessage::System(format!(
                    "👤 New character: {} ({})",
                    name,
                    role.as_deref().unwrap_or("unknown")
                )));
            }
            ToolCall::UpdateVar { name, value } => {
                ch.send(UiMessage::System(format!("  [{}={}]", name, value)));
            }
            ToolCall::RenderWidget {
                widget_type,
                title,
                data,
            } => {
                // Render inline widget in chat
                let title_str = title.as_deref().unwrap_or("");
                let widget_desc = format!("{:?}", widget_type);
                ch.send(UiMessage::System(format!(
                    "  [📊 {} — {}]",
                    widget_desc, title_str
                )));
                // TODO: render actual widget inline once chat supports it
            }
        }
    }
}

// ===== STATUS =====

fn send_status(state: &GameState, ch: &GameChannels) {
    let phase_str = match state.phase {
        GamePhase::TitleScreen => "Title",
        GamePhase::Dawn => "Dawn",
        GamePhase::Action => {
            if state.active_npc.is_some() {
                "Conversation"
            } else {
                "Action"
            }
        }
        GamePhase::Dusk => "Dusk",
        GamePhase::Downtime => "Downtime",
        GamePhase::Event(_) => "Event",
        GamePhase::CharacterCreation => "Create",
        GamePhase::ElectionNight => "Election",
        GamePhase::CareerEnd => "End",
    };

    ch.send(UiMessage::StatusUpdate {
        week: state.week,
        year: state.year,
        phase: phase_str.to_string(),
        ap_current: state.ap_current(),
        ap_max: state.ap_max(),
    });
}
