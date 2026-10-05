//! Finds variable and function names at a position in a line.

use std::collections::HashMap;

/// Marks the end of a stored name; PlantUML forbids `\0` in names for this reason.
const END: char = '\0';

#[derive(Default)]
pub struct Trie {
    children: HashMap<char, Trie>,
}

impl Trie {
    pub fn add(&mut self, name: &str) {
        let mut node = self;
        for c in name.chars().chain([END]) {
            node = node.children.entry(c).or_default();
        }
    }

    pub fn remove(&mut self, name: &str) {
        let mut node = self;
        for c in name.chars() {
            let Some(child) = node.children.get_mut(&c) else {
                return;
            };
            node = child;
        }
        node.children.remove(&END);
    }

    /// Follows `chars` from `position` as far as the stored names allow and returns that prefix if a name
    /// ends there, or `""`. It does not back off to a shorter name: PlantUML's lookup is greedy too.
    pub fn longest_match_starting_in(&self, chars: &[char], position: usize) -> String {
        let mut node = self;
        let mut matched = String::new();
        let mut position = position;
        loop {
            let Some(&c) = chars.get(position) else {
                return if node.children.contains_key(&END) {
                    matched
                } else {
                    String::new()
                };
            };
            match node.children.get(&c) {
                Some(child) if !child.children.is_empty() => {
                    matched.push(c);
                    node = child;
                    position += 1;
                }
                _ => {
                    return if node.children.contains_key(&END) {
                        matched
                    } else {
                        String::new()
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn matches_a_stored_name_at_the_position() {
        let mut trie = Trie::default();
        trie.add("$name");
        assert_eq!(
            trie.longest_match_starting_in(&chars("x $name y"), 2),
            "$name"
        );
        assert_eq!(trie.longest_match_starting_in(&chars("x $nam"), 2), "");
    }

    #[test]
    fn does_not_back_off_to_a_shorter_name() {
        let mut trie = Trie::default();
        trie.add("$a");
        trie.add("$abc");
        assert_eq!(trie.longest_match_starting_in(&chars("$abc"), 0), "$abc");
        assert_eq!(trie.longest_match_starting_in(&chars("$ab!"), 0), "");
        assert_eq!(trie.longest_match_starting_in(&chars("$a!"), 0), "$a");
    }

    #[test]
    fn removed_names_no_longer_match() {
        let mut trie = Trie::default();
        trie.add("$v");
        trie.remove("$v");
        assert_eq!(trie.longest_match_starting_in(&chars("$v"), 0), "");
    }
}
