pub fn parse_u64(s: &str) -> u64 {
    if s.is_empty() {
        return 0;
    }
    s.parse::<u64>().unwrap_or(0)
}

pub fn parse_u16(s: &str) -> i32 {
    let v: i32 = s.parse().unwrap_or(0);
    if v < 0 || v > 65535 {
        return 0;
    }
    v
}

pub fn parse_i10(s: &str) -> i32 {
    s.parse().unwrap_or(0)
}

pub fn is_auto_nickname_match(expected: &str, actual: &str) -> bool {
    if actual == expected {
        return true;
    }
    if !actual.starts_with(expected) {
        return false;
    }
    // Use .get() for char-safe slicing — avoids panic if expected.len()
    // falls on a non-char boundary (defensive; starts_with guarantees validity).
    let suffix = actual.get(expected.len()..).unwrap_or("");
    suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty()
}

pub fn split_command_rows(line: &str) -> Vec<String> {
    let space_idx = match line.find(' ') {
        Some(i) => i,
        None => return vec![line.to_string()],
    };

    let name = &line[..space_idx];
    let rest = &line[space_idx + 1..];

    if !rest.contains('|') {
        return vec![line.to_string()];
    }

    let parts: Vec<&str> = rest.split('|').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return vec![line.to_string()];
    }

    // TeamSpeak writes the parameters that every row shares once, in the first row. A later row
    // that leaves a key out carries the first row's value for it.
    // Only a named parameter is shared. A bare word is an option of its own row.
    let shared: Vec<(&str, &str)> = tokens(parts[0])
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect();
    let mut rows = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let mut row = format!("{name} {part}");
        if index > 0 {
            let own: Vec<&str> = tokens(part).map(|(key, _)| key).collect();
            for (key, value) in &shared {
                if !own.contains(key) {
                    row.push_str(&format!(" {key}={value}"));
                }
            }
        }
        rows.push(row);
    }
    rows
}

/// The tokens of one command row, still escaped. A bare word has no value.
fn tokens<'a>(part: &'a str) -> impl Iterator<Item = (&'a str, Option<&'a str>)> + 'a {
    part.split(' ')
        .filter(|t| !t.is_empty())
        .map(|t| match t.split_once('=') {
            Some((key, value)) => (key, Some(value)),
            None => (t, None),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_rows_inherit_the_shared_parameters() {
        let rows = split_command_rows(
            "notifycliententerview cfid=0 ctid=92 reasonid=0 clid=949 client_nickname=Ada             |clid=1253 client_nickname=Grace|clid=1255 client_nickname=Linus",
        );
        assert_eq!(rows.len(), 3);
        assert!(rows[1].contains("clid=1253"));
        assert!(rows[1].contains("client_nickname=Grace"));
        // The channel came once, in the first row, and belongs to all three.
        assert!(rows[1].contains("ctid=92"), "{}", rows[1]);
        assert!(rows[2].contains("ctid=92"), "{}", rows[2]);
        assert!(rows[1].contains("cfid=0"));
        // A row keeps its own value for a key it names itself.
        assert!(!rows[1].contains("clid=949"));
        assert!(!rows[1].contains("client_nickname=Ada"));
    }

    #[test]
    fn a_single_row_command_is_left_alone() {
        let line = "notifyclientmoved ctid=4 reasonid=0 clid=7";
        assert_eq!(split_command_rows(line), vec![line.to_string()]);
    }

    #[test]
    fn test_parse_u64() {
        assert_eq!(parse_u64(""), 0);
        assert_eq!(parse_u64("42"), 42);
        assert_eq!(parse_u64("0"), 0);
    }

    #[test]
    fn test_parse_u16() {
        assert_eq!(parse_u16("0"), 0);
        assert_eq!(parse_u16("65535"), 65535);
        assert_eq!(parse_u16("65536"), 0);
        assert_eq!(parse_u16("-1"), 0);
    }

    #[test]
    fn test_parse_i10() {
        assert_eq!(parse_i10("42"), 42);
        assert_eq!(parse_i10("0"), 0);
        assert_eq!(parse_i10("-1"), -1);
        assert_eq!(parse_i10("abc"), 0);
    }

    #[test]
    fn test_is_auto_nickname_match() {
        assert!(is_auto_nickname_match("user", "user"));
        assert!(is_auto_nickname_match("user", "user1"));
        assert!(is_auto_nickname_match("user", "user42"));
        assert!(!is_auto_nickname_match("user", "usera"));
        assert!(!is_auto_nickname_match("user", "other"));
    }

    #[test]
    fn test_split_command_rows() {
        let result = split_command_rows("cmd1 arg1|arg2|arg3");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "cmd1 arg1");
        assert_eq!(result[1], "cmd1 arg2");
        assert_eq!(result[2], "cmd1 arg3");
    }

    #[test]
    fn test_split_command_rows_no_pipe() {
        let result = split_command_rows("cmd1 arg1 arg2");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "cmd1 arg1 arg2");
    }

    #[test]
    fn test_split_command_rows_no_args() {
        let result = split_command_rows("cmd1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "cmd1");
    }
}
