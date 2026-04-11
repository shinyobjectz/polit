//! Gen-UI widget renderer — renders AI-generated widgets inline in chat.
//! The AI can produce bar charts, stat blocks, gauges, etc. via the
//! render_widget tool call.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ai::tools::WidgetType;
use crate::ui::theme;

/// Render a gen-UI widget as chat-compatible Lines.
/// Returns Vec<Line> that can be inserted into the chat stream.
pub fn render_widget_lines(
    widget_type: &WidgetType,
    title: Option<&str>,
    data: &serde_json::Value,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Title bar
    if let Some(t) = title {
        lines.push(Line::from(vec![
            Span::styled("┌─ ", Style::default().fg(theme::BORDER)),
            Span::styled(t.to_string(), Style::default().fg(theme::FG).bold()),
            Span::styled(" ─", Style::default().fg(theme::BORDER)),
        ]));
    }

    match widget_type {
        WidgetType::BarChart => render_bar_chart(&mut lines, data),
        WidgetType::Gauge => render_gauge(&mut lines, data),
        WidgetType::StatBlock => render_stat_block(&mut lines, data),
        WidgetType::Table => render_table(&mut lines, data),
        WidgetType::List => render_list(&mut lines, data),
        WidgetType::Alert => render_alert(&mut lines, data),
        WidgetType::Quote => render_quote(&mut lines, data),
        _ => {
            lines.push(Line::from(Span::styled(
                format!("  [{:?} widget]", widget_type),
                Style::default().fg(theme::FG_DIM),
            )));
        }
    }

    lines.push(Line::from("")); // spacing
    lines
}

fn render_bar_chart(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    if let Some(obj) = data.as_object() {
        let max_val = obj
            .values()
            .filter_map(|v| v.as_f64())
            .fold(0.0f64, f64::max)
            .max(1.0);

        for (key, val) in obj {
            let v = val.as_f64().unwrap_or(0.0);
            let bar_width = ((v / max_val) * 20.0) as usize;
            let bar = "█".repeat(bar_width);
            let empty = "░".repeat(20 - bar_width);
            lines.push(Line::from(vec![
                Span::styled(format!("  {:>12} ", key), Style::default().fg(theme::FG_DIM)),
                Span::styled(bar, Style::default().fg(theme::ACCENT_BLUE)),
                Span::styled(empty, Style::default().fg(theme::BG_HIGHLIGHT)),
                Span::styled(format!(" {}", v), Style::default().fg(theme::FG)),
            ]));
        }
    }
}

fn render_gauge(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    let value = data.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let max = data.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
    let label = data
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Progress");

    let pct = (value / max).min(1.0);
    let filled = (pct * 30.0) as usize;
    let bar = format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(30 - filled)
    );

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", label), Style::default().fg(theme::FG_DIM)),
        Span::styled(bar, Style::default().fg(theme::ACCENT_BLUE)),
        Span::styled(
            format!(" {:.0}%", pct * 100.0),
            Style::default().fg(theme::FG),
        ),
    ]));
}

fn render_stat_block(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    if let Some(obj) = data.as_object() {
        for (key, val) in obj {
            let val_str = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => format!("{}", val),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:>14}: ", key), Style::default().fg(theme::FG_DIM)),
                Span::styled(val_str, Style::default().fg(theme::FG)),
            ]));
        }
    }
}

fn render_table(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    if let Some(rows) = data.get("rows").and_then(|v| v.as_array()) {
        // Header
        if let Some(headers) = data.get("headers").and_then(|v| v.as_array()) {
            let header_text: Vec<String> = headers
                .iter()
                .map(|h| format!("{:>12}", h.as_str().unwrap_or("")))
                .collect();
            lines.push(Line::from(Span::styled(
                format!("  {}", header_text.join(" │ ")),
                Style::default().fg(theme::FG).bold(),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", "─".repeat(header_text.len() * 15)),
                Style::default().fg(theme::BORDER),
            )));
        }
        // Rows
        for row in rows {
            if let Some(cells) = row.as_array() {
                let cell_text: Vec<String> = cells
                    .iter()
                    .map(|c| format!("{:>12}", c.as_str().unwrap_or(&c.to_string())))
                    .collect();
                lines.push(Line::from(Span::styled(
                    format!("  {}", cell_text.join(" │ ")),
                    Style::default().fg(theme::FG_DIM),
                )));
            }
        }
    }
}

fn render_list(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let fallback = item.to_string();
            let text = item.as_str().unwrap_or(&fallback);
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme::ACCENT_BLUE)),
                Span::styled(text.to_string(), Style::default().fg(theme::FG)),
            ]));
        }
    }
}

fn render_alert(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let message = data
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (icon, color) = match level {
        "warning" => ("⚠", theme::WARNING),
        "error" => ("✗", theme::ACCENT),
        "success" => ("✓", theme::SUCCESS),
        _ => ("ℹ", theme::FG_DIM),
    };

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
        Span::styled(message.to_string(), Style::default().fg(color)),
    ]));
}

fn render_quote(lines: &mut Vec<Line<'static>>, data: &serde_json::Value) {
    let text = data
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let attribution = data
        .get("attribution")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    for line in text.lines() {
        lines.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(theme::ACCENT_BLUE)),
            Span::styled(line.to_string(), Style::default().fg(theme::FG).italic()),
        ]));
    }
    if !attribution.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("    — {}", attribution),
            Style::default().fg(theme::FG_DIM),
        )));
    }
}
