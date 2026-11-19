use super::Standards;
use std::fs;

const ORCHESTRATION_SKILL: &str = "orchestration";
const MARKDOWN_EMPHASIS: [char; 3] = ['*', '_', '`'];

/// One way of running agents the orchestration skill offers the owner, written there as a
/// sub-item such as `Economy: Cap at 25 agents, third-tier models, xhigh effort.`
pub struct Tier {
    pub name: String,
    pub line: String,
}

impl Tier {
    /// An answer to a question picks the tier when it names it as a word, in any case.
    pub fn is_named_in_answer(&self, answer: &str) -> bool {
        contains_word(&answer.to_lowercase(), &self.name.to_lowercase())
    }

    /// A prompt picks the tier when it opens with the tier's name or calls it a tier, in any case.
    pub fn is_named_in_prompt(&self, prompt: &str) -> bool {
        let prompt = prompt.trim().to_lowercase();
        let name = self.name.to_lowercase();
        let opens_with_it = prompt.starts_with(&name) && is_word_edge(prompt[name.len()..].chars().next());
        opens_with_it || contains_word(&prompt, &format!("{name} tier"))
    }
}

/// The tiers in the orchestration skill: its indented list items written as `Name: what it means`.
pub fn offered(standards: &Standards) -> Vec<Tier> {
    standards
        .skills
        .iter()
        .find(|skill| skill.name == ORCHESTRATION_SKILL)
        .and_then(|skill| fs::read_to_string(skill.file()).ok())
        .map(|text| parse(&text))
        .unwrap_or_default()
}

fn parse(skill: &str) -> Vec<Tier> {
    skill.lines().filter_map(tier_on).collect()
}

fn tier_on(line: &str) -> Option<Tier> {
    let item = line.strip_prefix(char::is_whitespace)?.trim_start().strip_prefix("- ")?.trim();
    let (name, _) = item.split_once(": ")?;
    let name = name.trim().trim_matches(MARKDOWN_EMPHASIS.as_slice());
    (!name.is_empty()).then(|| Tier {
        name: name.to_string(),
        line: item.to_string(),
    })
}

fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + word.len()..].chars().next();
        is_word_edge(before) && is_word_edge(after)
    })
}

fn is_word_edge(neighbour: Option<char>) -> bool {
    !neighbour.is_some_and(char::is_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKILL: &str = "---\nname: orchestration\n---\n\n- Before starting, present the following options:\n  - **Cheap**: one agent.\n  - Cheap (Swarm): many agents.\n- Only use agents for:\n  - Mapping a codebase.\n";

    fn tier(name: &str) -> Tier {
        Tier {
            name: name.to_string(),
            line: String::new(),
        }
    }

    #[test]
    fn tiers_are_the_named_sub_items_of_the_skill() {
        let names: Vec<String> = parse(SKILL).into_iter().map(|tier| tier.name).collect();
        assert_eq!(names, ["Cheap", "Cheap (Swarm)"]);
    }

    #[test]
    fn an_answer_names_a_tier_anywhere_and_a_prompt_only_up_front_or_as_a_tier() {
        let balanced = tier("Balanced");
        assert!(balanced.is_named_in_answer("Balanced (Recommended)"));
        assert!(!balanced.is_named_in_answer("Unbalanced"));

        assert!(balanced.is_named_in_prompt("balanced, and keep it short"));
        assert!(balanced.is_named_in_prompt("Do it on the Balanced tier."));
        assert!(!balanced.is_named_in_prompt("Keep the tree balanced while you refactor"));
        assert!(!balanced.is_named_in_prompt("balancedness"));
    }
}
