use crate::{message::Message, result::Violation, rule::Rule};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::Level;

/// BodyMaxLength represents the body-max-length rule.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TicketId {
    /// Level represents the level of the rule.
    ///
    // Note that currently the default literal is not supported.
    // See: https://github.com/serde-rs/serde/issues/368
    level: Option<Level>,

    /// Whether there should be just one occurence of ticket ID in the message
    unique: bool,
    /// Whether ticket ID can be inside the subject
    subject: bool,
    /// Whether ticket ID can be inside the body
    body: bool,
    /// Whether ticket ID should be the last line in the body, else it can be anywhere in the body
    body_last_line: bool,
}

static JIRA_TICKET_REGEX: &str = r"#[A-Z]+-\d+";

/// BodyMaxLength represents the body-max-length rule.
impl Rule for TicketId {
    const NAME: &'static str = "ticket-id";
    const LEVEL: Level = Level::Error;

    fn message(&self, _message: &Message) -> String {
        format!("Ticket ID is missing in either subject or body of commit message! It should be on last line if inside body, or at the end of subject!")
    }

    fn validate(&self, message: &Message) -> Option<Violation> {
        let mut match_found = 0;
        let mut subject_has_ticket = false;
        let mut last_line_has_ticket = false;
        let regex = match Regex::new(JIRA_TICKET_REGEX) {
            Ok(regex) => regex,
            Err(error) => {
                return Some(Violation {
                    level: self.level.unwrap_or(Self::LEVEL),
                    message: format!("Invalid regex {JIRA_TICKET_REGEX}: {}", error),
                })
            }
        };

        if self.subject && regex.is_match(message.subject.clone().unwrap_or_default().as_str()) {
            match_found += 1;
            subject_has_ticket = true;
        }

        if self.body {
            let body = message.body.clone().unwrap_or(String::new());
            let last_line = body.lines().last();

            for line in body.lines() {
                if regex.is_match(line) {
                    match_found += 1;
                    // Check if this is the last line in the body
                    if self.body_last_line && last_line == Some(&line) {
                        last_line_has_ticket = true;
                    }
                }
            }
        }

        // Error messages
        if match_found == 0 && (self.body || self.subject) {
            return Some(Violation {
                level: self.level.unwrap_or(Self::LEVEL),
                message: format!("Ticket ID is missing in either subject or body of commit message! It should be on last line if inside body, or at the end of subject!"),
            });
        }

        if self.unique {
            if last_line_has_ticket && match_found > 1 {
                return Some(Violation {
                    level: self.level.unwrap_or(Self::LEVEL),
                    message: format!("Ticket ID is duplicated in body of commit message!"),
                });
            }

            if match_found > 1 {
                return Some(Violation {
                    level: self.level.unwrap_or(Self::LEVEL),
                    message: format!(
                        "Ticket ID is duplicated in either subject or body of commit message!"
                    ),
                });
            }
        }

        if self.body_last_line {
            if !last_line_has_ticket
                && match_found >= 1
                && !message.body.clone().unwrap_or_default().is_empty()
                && !subject_has_ticket
            {
                return Some(Violation {
                    level: self.level.unwrap_or(Self::LEVEL),
                    message: format!("Ticket ID should be on last line in body!"),
                });
            }
        }

        None
    }
}

/// Default implementation of TicketId.
impl Default for TicketId {
    fn default() -> Self {
        Self {
            level: Some(Self::LEVEL),
            unique: false,
            subject: true,
            body: true,
            body_last_line: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_body() {
        let rule = TicketId {
            ..Default::default()
        };
        let message = Message {
            body: Some("Hello world  #BOS-494".to_string()),
            description: Some("broadcast $destroy event on scope destruction".to_string()),
            footers: None,
            r#type: Some("feat".to_string()),
            raw: "feat(scope): broadcast $destroy event on scope destruction #BOS-494

Hey!"
                .to_string(),
            scope: Some("scope".to_string()),
            subject: Some("feat(scope): broadcast $destroy event on scope destruction".to_string()),
        };

        assert!(rule.validate(&message).is_none());
    }

    #[test]
    fn test_subject() {
        let rule = TicketId {
            ..Default::default()
        };
        let message = Message {
            body: Some("Hello, I'm a long body".to_string()),
            description: None,
            footers: None,
            r#type: Some("feat".to_string()),
            raw: "feat(scope): broadcast $destroy event on scope destruction #BOS-494

Hello, I'm a long body"
                .to_string(),
            scope: Some("scope".to_string()),
            subject: Some(
                "feat(scope): broadcast $destroy event on scope destruction #BOS-494".to_string(),
            ),
        };
        assert!(rule.validate(&message).is_none());
    }

    #[test]
    fn test_body() {
        let rule = TicketId {
            ..Default::default()
        };
        let message = Message {
            body: Some("Hello, I'm a long body  #BOS-494".to_string()),
            description: None,
            footers: None,
            r#type: Some("feat".to_string()),
            raw: "feat(scope): broadcast $destroy event on scope destruction

Hello, I'm a long body"
                .to_string(),
            scope: Some("scope".to_string()),
            subject: Some(
                "feat(scope): broadcast $destroy event on scope destruction #BOS-494".to_string(),
            ),
        };
        assert!(rule.validate(&message).is_none());
    }

    #[test]
    fn test_subject_body_missing() {
        let rule = TicketId {
            ..Default::default()
        };
        let message = Message {
            body: Some("Hello, I'm a long body".to_string()),
            description: None,
            footers: None,
            r#type: Some("feat".to_string()),
            raw: "feat(scope): broadcast $destroy event on scope destruction

Hello, I'm a long body"
                .to_string(),
            scope: Some("scope".to_string()),
            subject: None,
        };
        let violation = rule.validate(&message);
        assert!(violation.is_some());
        assert_eq!(violation.clone().unwrap().level, Level::Error);
    }

    #[test]
    fn test_body_last_line() {
        let rule = TicketId {
            ..Default::default()
        };
        let message = Message {
            body: Some(
                "feat(scope): broadcast $destroy event on scope destruction

This body has more lines #BOS-494
But the last one is missing
the ticket id."
                    .to_string(),
            ),
            description: None,
            footers: None,
            r#type: Some("feat".to_string()),
            raw: "feat(scope): broadcast $destroy event on scope destruction

This body has more lines #BOS-494
But the last one is missing
the ticket id."
                .to_string(),
            scope: Some("scope".to_string()),
            subject: None,
        };
        let violation = rule.validate(&message);
        assert!(violation.is_some());
        assert_eq!(violation.clone().unwrap().level, Level::Error);
    }
}
