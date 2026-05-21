pub mod git;
pub mod list;
pub mod search;
pub mod symbol;
pub mod tree;
pub mod view;


/// A parsed command from a Feishu message.
#[derive(Debug)]
pub struct ParsedCommand {
    /// The command name (e.g., "view", "tree").
    pub name: String,
    /// Positional arguments.
    pub args: Vec<String>,
    /// The raw input text.
    pub raw: String,
}

/// Parse a raw text message into a command.
///
/// Commands start with `/` and have whitespace-separated arguments.
/// Quoted strings are treated as single arguments.
pub fn parse_command(text: &str) -> Option<ParsedCommand> {
    let text = text.trim();
    if !text.starts_with('/') {
        return None;
    }

    let text = &text[1..]; // strip leading `/`
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in text.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() {
        return None;
    }

    let name = parts.remove(0).to_lowercase();
    Some(ParsedCommand {
        name,
        args: parts,
        raw: text.to_string(),
    })
}
