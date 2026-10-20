//! TeamSpeak channel names that draw a divider. A name like `[*spacer1]—` or
//! `[cspacer]«» VIP «»` names no place to talk: it rules a line or sets a heading across the
//! tree. The tag holds an alignment and an id that only keeps the name unique.

use crate::model::Spacer;

/// Splits a channel name into the divider it draws and the label it shows.
pub fn split(name: &str) -> (Option<Spacer>, String) {
    let Some(rest) = name.strip_prefix('[') else {
        return (None, name.to_owned());
    };
    let Some(close) = rest.find(']') else {
        return (None, name.to_owned());
    };
    let tag = &rest[..close];
    let label = &rest[close + 1..];
    let spacer = if tag.strip_prefix("*spacer").is_some() {
        Spacer::Fill
    } else if tag.strip_prefix("cspacer").is_some() {
        Spacer::Center
    } else if tag.strip_prefix("rspacer").is_some() {
        Spacer::Right
    } else if tag.strip_prefix("lspacer").is_some() || tag.strip_prefix("spacer").is_some() {
        Spacer::Left
    } else {
        return (None, name.to_owned());
    };
    (Some(spacer), label.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filled_spacer_keeps_the_label_it_repeats() {
        assert_eq!(split("[*spacer1]—"), (Some(Spacer::Fill), "—".to_owned()));
    }

    #[test]
    fn reads_every_alignment() {
        assert_eq!(split("[cspacer]Title").0, Some(Spacer::Center));
        assert_eq!(split("[rspacer3]Title").0, Some(Spacer::Right));
        assert_eq!(split("[lspacer3]Title").0, Some(Spacer::Left));
        assert_eq!(split("[spacer9]Title").0, Some(Spacer::Left));
    }

    #[test]
    fn keeps_the_heading_a_spacer_carries() {
        assert_eq!(
            split("[cspacer]«» Server Management «»"),
            (Some(Spacer::Center), "«» Server Management «»".to_owned())
        );
    }

    #[test]
    fn a_spacer_with_no_label_draws_a_gap() {
        assert_eq!(split("[*spacer2de]"), (Some(Spacer::Fill), String::new()));
    }

    #[test]
    fn an_ordinary_name_is_left_whole() {
        assert_eq!(split("Lobby"), (None, "Lobby".to_owned()));
        assert_eq!(
            split("Baggerfahrer Führerhaus"),
            (None, "Baggerfahrer Führerhaus".to_owned())
        );
    }

    #[test]
    fn a_bracket_that_names_no_spacer_is_part_of_the_name() {
        assert_eq!(
            split("[Important] Channel"),
            (None, "[Important] Channel".to_owned())
        );
        assert_eq!(split("[unclosed"), (None, "[unclosed".to_owned()));
    }

    #[test]
    fn a_spacer_may_still_be_a_place_to_talk() {
        // This server centres its lobby, and people sit in it.
        assert_eq!(
            split("[cspacer]● Lobby"),
            (Some(Spacer::Center), "● Lobby".to_owned())
        );
    }
}
