use polit::devtools::frame_dump::buffer_to_text_lines;
use ratatui::backend::TestBackend;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

#[test]
fn poldev_frame_dump_dumps_buffer_as_normalized_text_lines() {
    let backend = TestBackend::new(12, 3);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            frame.render_widget(Paragraph::new("hello\nworld"), frame.area());
        })
        .unwrap();

    let lines = buffer_to_text_lines(terminal.backend().buffer());

    assert_eq!(
        lines,
        vec!["hello".to_string(), "world".to_string(), "".to_string()]
    );
}

#[test]
fn poldev_frame_dump_preserves_wide_glyphs() {
    let backend = TestBackend::new(4, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            frame.render_widget(Paragraph::new("界"), frame.area());
        })
        .unwrap();

    let lines = buffer_to_text_lines(terminal.backend().buffer());

    assert_eq!(lines, vec!["界".to_string()]);
}
