use ratatui::buffer::Buffer;

pub fn buffer_to_text_lines(buffer: &Buffer) -> Vec<String> {
    let width = buffer.area.width as usize;
    let height = buffer.area.height as usize;

    (0..height)
        .map(|row| {
            let start = row * width;
            let end = start + width;
            let line = buffer.content[start..end]
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            line.trim_end().to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::buffer_to_text_lines;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn trims_trailing_whitespace_per_line() {
        let buffer = Buffer::with_lines(["hello  ", "world\t"]);
        let lines = buffer_to_text_lines(&buffer);

        assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn preserves_blank_lines_as_empty_strings() {
        let buffer = Buffer::with_lines(["hello", ""]);
        let lines = buffer_to_text_lines(&buffer);

        assert_eq!(lines, vec!["hello".to_string(), "".to_string()]);
    }

    #[test]
    fn handles_empty_buffers() {
        let buffer = Buffer::empty(Rect::new(0, 0, 0, 0));
        let lines = buffer_to_text_lines(&buffer);

        assert!(lines.is_empty());
    }
}
