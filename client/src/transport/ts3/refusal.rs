//! What to tell the person when a TeamSpeak server refuses the connection.

/// Where the security level is raised.
const RAISE: &str = "Raise it in Settings → Identity.";

/// What a server's refusal means for the person who reads it. `level` is the security level the
/// identity in use stands at.
pub fn explain(message: &str, level: i32) -> String {
    let said = message.trim();
    if said.is_empty() {
        return "The server refused the connection".to_owned();
    }
    if said.to_lowercase().contains("security level") {
        return format!("The server refused: {said}. This identity is at level {level}. {RAISE}");
    }
    format!("The server refused: {said}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_at_the_identity_when_the_level_is_too_low() {
        let said = "it asks for identity security level 23";
        assert_eq!(
            explain(said, 9),
            format!("The server refused: {said}. This identity is at level 9. {RAISE}")
        );
    }

    #[test]
    fn reads_the_level_whatever_the_wording() {
        assert!(explain("Security Level too low", 8).ends_with(RAISE));
        assert!(explain("needed identity security level: 24", 12).ends_with(RAISE));
    }

    #[test]
    fn passes_another_refusal_through() {
        let said = "invalid server password";
        assert_eq!(explain(said, 9), format!("The server refused: {said}"));
    }

    #[test]
    fn stands_in_for_a_refusal_that_gives_no_reason() {
        assert_eq!(explain("  ", 9), "The server refused the connection");
    }
}
