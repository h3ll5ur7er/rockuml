//! Cuts the source into `@start...`/`@end...` blocks, honouring `@pause`, `@unpause`, `@append` and `!exit`.

use super::reader::{
    ReadFilterAddConfig, ReadFilterMergeLines, ReadLine, ReadLineReader, UncommentReadLine,
    remove_yaml_header,
};
use super::start_utils;
use crate::text::StringLocated;

/// Each block runs from its `@start` line to its `@end` line inclusive.
pub fn extract_blocks(
    source: &str,
    description: &str,
    config: Vec<String>,
) -> Vec<Vec<StringLocated>> {
    let uncomment =
        UncommentReadLine::new(Box::new(ReadLineReader::new(source, description, None)));
    let paused_reader = uncomment.pause_switch();
    let mut reader = ReadFilterMergeLines::new(Box::new(ReadFilterAddConfig::new(
        Box::new(uncomment),
        config,
    )));

    let mut blocks = Vec::new();
    let mut current: Option<Vec<StringLocated>> = None;
    let mut paused = false;
    while let Some(line) = reader.read_line() {
        let text = line.text();
        if start_utils::is_start_directive(text) {
            current = Some(Vec::new());
            paused = false;
        }
        if start_utils::is_pause_directive(text) || start_utils::is_exit(text) {
            paused = true;
            paused_reader.set(true);
        }
        if let Some(lines) = current.as_mut() {
            if !paused {
                lines.push(line.clone());
            } else if let Some(appended) = start_utils::possible_append(&line) {
                lines.push(appended);
            }
        }
        if start_utils::is_unpause_directive(text) {
            paused = false;
            paused_reader.set(false);
        }
        if start_utils::is_end_directive(text)
            && let Some(mut lines) = current.take()
        {
            if paused {
                lines.push(line);
            }
            blocks.push(remove_yaml_header(lines));
            paused_reader.set(false);
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(block: &[StringLocated]) -> Vec<&str> {
        block.iter().map(StringLocated::text).collect()
    }

    #[test]
    fn text_outside_blocks_is_ignored() {
        let blocks = extract_blocks("intro\n@startuml\nA -> B\n@enduml\noutro", "t", vec![]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(texts(&blocks[0]), ["@startuml", "A -> B", "@enduml"]);
    }

    #[test]
    fn paused_lines_are_skipped_except_appends() {
        let source = "@startuml\nA\n@pause\nB\n@append C\n@unpause\nD\n@enduml";
        let blocks = extract_blocks(source, "t", vec![]);
        assert_eq!(texts(&blocks[0]), ["@startuml", "A", "C", "D", "@enduml"]);
    }

    #[test]
    fn exit_ends_the_block_content() {
        let blocks = extract_blocks("@startuml\nA\n!exit\nB\n@enduml", "t", vec![]);
        assert_eq!(texts(&blocks[0]), ["@startuml", "A", "@enduml"]);
    }
}
