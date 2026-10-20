//! Built-in English documentation, compiled into the executable so that an
//! AI agent needs nothing but the exe: `any_txt_parser help [topic]` and the
//! MCP `help` tool print these texts.

pub struct Topic {
    pub name: &'static str,
    pub summary: &'static str,
    pub text: &'static str,
}

pub const TOPICS: &[Topic] = &[
    Topic {
        name: "overview",
        summary: "what the program is for, the commands, how to start",
        text: include_str!("../docs/help/overview.md"),
    },
    Topic {
        name: "extract",
        summary: "pull matching data out of files (rules on the command line)",
        text: include_str!("../docs/help/extract.md"),
    },
    Topic {
        name: "rules",
        summary: "rule shorthand, rule objects, selectors per format, transforms",
        text: include_str!("../docs/help/rules.md"),
    },
    Topic {
        name: "recon",
        summary: "inspect files first: format, encoding, shape, sample lines",
        text: include_str!("../docs/help/recon.md"),
    },
    Topic {
        name: "navigation",
        summary: "map, symbols, find, lines, search, todos, changes, nav",
        text: include_str!("../docs/help/navigation.md"),
    },
    Topic {
        name: "mcp",
        summary: "run as an MCP server; client configuration; tool list",
        text: include_str!("../docs/help/mcp.md"),
    },
    Topic {
        name: "reference",
        summary: "full reference of the template DSL and the Markdown report",
        text: include_str!("../HELP.md"),
    },
];

/// The text for `topic` (case-insensitive; `None`/empty means overview),
/// or an error listing the topics.
pub fn topic(name: Option<&str>) -> Result<&'static str, String> {
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("overview");
    TOPICS
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
        .map(|t| t.text)
        .ok_or_else(|| format!("unknown help topic '{}'.\n\n{}", name, index()))
}

/// One line per topic.
pub fn index() -> String {
    let mut out = String::from("Help topics (any_txt_parser help <topic>):\n");
    for t in TOPICS {
        out.push_str(&format!("  {:<11} {}\n", t.name, t.summary));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_is_reachable_and_non_empty() {
        for t in TOPICS {
            let text = topic(Some(t.name)).unwrap();
            assert!(text.len() > 200, "topic {} is too short", t.name);
            assert!(index().contains(t.name));
        }
        assert_eq!(topic(None).unwrap(), TOPICS[0].text);
        assert_eq!(topic(Some("  MCP ")).unwrap(), topic(Some("mcp")).unwrap());
        let error = topic(Some("nope")).unwrap_err();
        assert!(error.contains("unknown help topic 'nope'") && error.contains("navigation"));
    }
}
