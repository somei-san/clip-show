pub fn truncate_text(text: &str, max_width: usize, max_lines: usize) -> String {
    let mut lines: Vec<String> = split_non_trailing_lines(text)
        .into_iter()
        .map(|line| truncate_line(line, max_width))
        .collect();

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            *last = append_ellipsis(last, max_width);
        }
    }

    lines.join("\n")
}

fn truncate_line(line: &str, max_width: usize) -> String {
    let count = line.chars().count();
    if count <= max_width {
        return line.to_string();
    }

    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }

    let kept: String = line.chars().take(max_width - 3).collect();
    format!("{kept}...")
}

fn append_ellipsis(line: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }

    let current_len = line.chars().count();
    if current_len + 3 <= max_width {
        return format!("{line}...");
    }

    let kept: String = line.chars().take(max_width - 3).collect();
    format!("{kept}...")
}

pub fn split_non_trailing_lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text
        .split_terminator('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect();

    while matches!(lines.last(), Some(last) if last.trim().is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        lines.push("");
    }
    lines
}

#[cfg(test)]
pub(crate) fn line_display_units(line: &str) -> f64 {
    let units: f64 = line
        .chars()
        .map(|c| if c.is_ascii() { 1.0 } else { 2.0 })
        .sum();
    units.max(1.0)
}

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn truncates_single_long_line() {
        let input = "abcdefghijklmnopqrstuvwxyz";
        assert_eq!(truncate_text(input, 10, 5), "abcdefg...");
    }

    #[test]
    fn truncates_lines_count_and_adds_ellipsis_to_last_line() {
        let input = "line1\nline2\nline3\nline4\nline5\nline6";
        assert_eq!(
            truncate_text(input, 100, 5),
            "line1\nline2\nline3\nline4\nline5..."
        );
    }

    #[test]
    fn handles_utf8_by_char_count() {
        let input = "あいうえおかきくけこ";
        assert_eq!(truncate_text(input, 6, 5), "あいう...");
    }
}
