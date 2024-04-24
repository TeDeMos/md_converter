use std::iter;

use temp_block::TempBlock;

use crate::ast::Pandoc;
use crate::traits::AstReader;

mod temp_block;

pub struct MdReader;

impl AstReader for MdReader {
    type ReadError = !;

    fn read(source: &str) -> Result<Pandoc, Self::ReadError> {
        let mut current = TempBlock::default();
        let mut finished: Vec<TempBlock> = Vec::new();
        for line in source.lines() {
            current.next(line, &mut finished);
        }
        let result =
            finished.into_iter().chain(iter::once(current)).filter_map(TempBlock::finish).collect();
        Ok(Pandoc { blocks: result, ..Default::default() })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::process::{Command, Stdio};

    use super::*;
    use crate::ast::*;

    fn test(examples: Vec<&str>, offset: usize) {
        let mut results = Vec::new();
        for (i, e) in examples.into_iter().enumerate() {
            let mut child = Command::new("pandoc")
                .args(["-f", "gfm", "-t", "json"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.as_mut().unwrap().write_all(e.as_bytes()).unwrap();
            let number = i + offset;
            let expected = if number == 68 {
                Pandoc {
                    blocks: vec![Block::HorizontalRule, Block::HorizontalRule],
                    ..Default::default()
                }
            } else {
                serde_json::from_str(
                    std::str::from_utf8(&child.wait_with_output().unwrap().stdout).unwrap(),
                )
                .unwrap()
            };
            let result = MdReader::read(e).into_ok();
            if result.blocks == expected.blocks {
                println!("\x1b[32mExample {} : success", number);
                println!("Input:\n{}", e);
                println!("Output:\n{:?}", result);
            } else {
                println!("\x1b[31mExample {} : failure", number);
                println!("Input:\n{}", e);
                println!("Output:\n{:?}", result);
                println!("Expected: \n{:?}", expected);
                results.push(number)
            }
        }
        if !results.is_empty() {
            panic!("Tests {:?} failed", results)
        }
    }

    #[test]
    fn test_thematic_break() {
        test(
            vec![
                "***\n---\n___", "+++", "===", "--\n**\n__", " ***\n  ***\n   ***", "    ***",
                "Foo\n    ***", "_____________________________________", " - - -",
                " **  * ** * ** * **", "-     -      -      -", "- - - -    ",
                "_ _ _ _ a\n\na------\n\n---a---", " *-*", "- foo\n***\n- bar", "Foo\n***\nbar",
                "Foo\n---\nbar", "* Foo\n* * *\n* Bar", "- Foo\n- * * *",
            ],
            13,
        );
    }

    #[test]
    fn test_atx_header() {
        test(
            vec![
                "# foo\n## foo\n### foo\n#### foo\n##### foo\n###### foo", "####### foo",
                "#5 bolt\n\n#hashtag", "\\## *bar* \\*baz\\*",
                "#                  foo                     ", " ### foo\n  ## foo\n   # foo",
                "    # foo", "foo\n    # bar", "## foo ##\n  ###   bar    ###",
                "# foo ##################################\n##### foo ##", "### foo ###     ",
                "### foo ### b", "# foo#", "### foo \\###\n## foo #\\##\n# foo \\#",
                "****\n## foo\n****", "Foo bar\n# baz\nBar foo", "## \n#\n### ###",
            ],
            32,
        );
    }

    #[test]
    fn test_setext_header() {
        test(
            vec![
                "Foo *bar*\n=========\n\nFoo *bar*\n---------", "Foo *bar\nbaz*\n====",
                "  Foo *bar\nbaz*\t\n====", "Foo\n-------------------------\n\nFoo\n=",
                "   Foo\n---\n\n  Foo\n-----\n\n  Foo\n  ===", "    Foo\n    ---\n\n    Foo\n---",
                "Foo\n   ----      ", "Foo\n    ---", "Foo\n= =\n\nFoo\n--- -", "Foo  \n-----",
                "Foo\\\n----", "`Foo\n----\n`\n\n<a title=\"a lot\n---\nof dashes\"/>",
                "> Foo\n---", "> foo\nbar\n===", "- Foo\n---", "Foo\nBar\n---",
                "---\nFoo\n---\nBar\n---\nBaz", "\n====", "---\n---", "- foo\n-----",
                "    foo\n---", "> foo\n-----", "\\> foo\n------", "Foo\n\nbar\n---\nbaz",
                "Foo\nbar\n\n---\n\nbaz", "Foo\nbar\n* * *\nbaz", "Foo\nbar\n\\---\nbaz",
            ],
            50,
        )
    }

    #[test]
    fn test_indented_code_block() {
        test(
            vec![
                "    a simple\n      indented code block", "  - foo\n\n    bar",
                "1.  foo\n\n    - bar", "    <a/>\n    *hi*\n\n    - one",
                "    chunk1\n\n    chunk2\n\n\n    chunk3", "    chunk1\n      \n      chunk2",
                "Foo\n    bar", "    foo\nbar",
                "# Heading\n    foo\nHeading\n------\n    foo\n----", "        foo\n    bar",
                "    \n    foo\n    ", "    foo  ",
            ],
            77,
        )
    }

    #[test]
    fn test_fenced_code_block() {
        test(
            vec![
                "```\n<\n >\n```", "~~~\n<\n >\n~~~", "``\nfoo\n``", "```\naaa\n~~~\n```",
                "~~~\n\naaa\n```\n~~~", "````\naaa\n```\n``````", "~~~~\naaa\n~~~\n~~~~", "```",
                "`````\n\n```\naaa", "> ```\n> aaa\n\nbbb", "```\n\n  \n```", "```\n```",
                " ```\n aaa\naaa\n```", "  ```\naaa\n  aaa\naaa\n  ```",
                "   ```\n   aaa\n    aaa\n  aaa\n   ```", "    ```\n    aaa\n    ```",
                "```\naaa\n  ```", "   ```\naaa\n  ```", "```\naaa\n    ```", "``` ```\naaa",
                "~~~~~~\naaa\n~~~ ~~", "foo\n```\nbar\n```\nbaz", "foo\n---\n~~~\nbar\n~~~\n# baz",
                "```ruby\ndef foo(x)\n  return 3\nend\n```",
                "~~~~    ruby startline=3 $%@#$\ndef foo(x)\n  return 3\nend\n~~~~~~~",
                "````;\n````", "``` aa ```\nfoo", "~~~ aa ``` ~~~\nfoo\n~~~", "```\n``` aaa\n```",
            ],
            89,
        )
    }

    #[test]
    fn test_table() {
        test(
            vec![
                "| foo | bar |\n| --- | --- |\n| baz | bim |",
                "| abc | defghi |\n:-: | -----------:\nbar | baz",
                "| f\\|oo  |\n| ------ |\n| b `\\|` az |\n| b **\\|** im |",
                "| abc | def |\n| --- | --- |\n| bar | baz |\n> bar",
                "| abc | def |\n| --- | --- |\n| bar | baz |\nbar\n\nbar",
                "| abc | def |\n| --- |\n| bar |",
                "| abc | def |\n| --- | --- |\n| bar |\n| bar | baz | boo |",
                "| abc | def |\n| --- | --- |",
            ],
            198,
        )
    }

    #[test]
    fn test_quote_block() {
        test(
            vec![
                "> # Foo\n> bar\n> baz", "># Foo\n>bar\n> baz", "   > # Foo\n   > bar\n > baz",
                "    > # Foo\n    > bar\n    > baz", "> # Foo\n> bar\nbaz", "> bar\nbaz\n> foo",
                "> foo\n---", "> - foo\n- bar", ">     foo\n    bar", "> ```\nfoo\n```",
                "> foo\n    - bar", ">", ">\n>  \n> ", ">\n> foo\n>  ", "> foo\n\n> bar",
                "> foo\n> bar", "> foo\n>\n> bar", "foo\n> bar", "> aaa\n***\n> bbb", "> bar\nbaz",
                "> bar\n\nbaz", "> bar\n>\nbaz", "> > > foo\nbar", ">>> foo\n> bar\n>>baz",
                ">     code\n\n>    not code",
            ],
            206,
        )
    }
}
