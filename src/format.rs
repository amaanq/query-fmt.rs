use tree_sitter::{Node, Parser};

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crate::args::Args;

fn check_parent(parent_kind: &str, node: Node) -> bool {
    node.parent()
        .map_or(false, |parent_node| parent_node.kind() == parent_kind)
}

fn get_len(source: &str) -> usize {
    source
        .chars()
        .filter(|char| char != &'\n' && char != &' ' && char != &'\t' && char != &'\r')
        .count()
}

pub fn format_file(path: &Path, mut parser: Parser, args: &Args) {
    let mut file = File::open(path).expect("Unable to open the file");
    println!("File: {}", path.display());
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read the file");
    let source_code = &contents;
    let original_len = get_len(source_code);
    let tree = parser.parse(source_code, None).unwrap();
    let mut comment_before = false;
    let mut output = String::new();
    let mut reached_root = false;
    let mut cursor = tree.walk();
    let mut nesting_level = 0;
    let mut indent_level = 0;
    while !reached_root {
        adapt_indent_level(&cursor, &mut indent_level);

        match cursor.node().kind() {
            "field_definition" => {
                output.push('\n');
                output.push_str(&" ".repeat(indent_level));
            }
            "predicate" => {
                output.push('\n');
                output.push_str(&" ".repeat(indent_level));
            }
            _ => {}
        }
        if cursor.node().kind() == "comment" && !comment_before {
            output.push('\n');
        }
        if cursor.node().kind() == "comment" {
            comment_before = true;
        } else {
            comment_before = false;
        }
        if nesting_level == 1 {
            output.push('\n');
            if cursor.node().kind() != "comment" {
                output.push('\n');
            }
        }
        if cursor.node().kind() == "capture" && !check_parent("parameters", cursor.node()) {
            output.push(' ');
        }

        indent_list_contents(&cursor, &mut output, indent_level);

        if cursor.node().kind() == "]" && check_parent("list", cursor.node()) {
            output.push('\n');
            output.push_str(&" ".repeat(indent_level));
        }

        if cursor.node().kind() == "identifier"
            && check_parent("anonymous_node", cursor.node())
            && !check_parent("list", cursor.node().parent().unwrap())
            && !check_parent("grouping", cursor.node().parent().unwrap())
        {
            output.push('\n');
            output.push_str(&" ".repeat(indent_level));
        }

        add_spacing_around_parameters(&cursor, &mut output);

        if check_parent("named_node", cursor.node()) && cursor.node().kind() == "named_node" {
            output.push('\n');
            output.push_str(&" ".repeat(indent_level));
        }

        push_text_to_output(&cursor, &mut output, source_code);

        add_space_after_colon(&cursor, &mut output);

        if cursor.goto_first_child() {
            nesting_level += 1;
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        let mut retracing = true;
        while retracing {
            if !cursor.goto_parent() {
                retracing = false;
                reached_root = true;
            } else {
                nesting_level -= 1;
            }
            if cursor.goto_next_sibling() {
                retracing = false;
            }
        }
    }
    output = output.trim().to_owned();
    if get_len(&output) != original_len {
        println!(
            "There was an error parsing your code.
Not applying formatting.
Open an issue."
        );
    } else if args.preview {
        println!("{output}");
    } else if !args.preview {
        let mut new_file = File::create(path).expect("Unable to open the file");
        writeln!(&mut new_file, "{output}").unwrap();
    }
}

fn add_spacing_around_parameters(cursor: &tree_sitter::TreeCursor, output: &mut String) {
    if check_parent("parameters", cursor.node()) {
        output.push(' ')
    }
}

fn push_text_to_output(
    cursor: &tree_sitter::TreeCursor,
    output: &mut String,
    source_code: &String,
) {
    if cursor.node().kind() == "escape_sequence" {
        return;
    }
    if cursor.node().child_count() == 0 && cursor.node().kind() != "\""
        || cursor.node().kind() == "string"
    {
        output.push_str(cursor.node().utf8_text(source_code.as_bytes()).unwrap());
    }
    // Directly add list item text
    if cursor.node().kind() == "anonymous_node" && check_parent("list", cursor.node()) {
        output.push_str(cursor.node().utf8_text(source_code.as_bytes()).unwrap());
    }
    if cursor.node().kind() == "identifier"
        && check_parent("anonymous_node", cursor.node())
        // Don't add list item text twice
        && !check_parent("list", cursor.node().parent().unwrap())
    {
        output.push_str(cursor.node().utf8_text(source_code.as_bytes()).unwrap());
    }
}

fn add_space_after_colon(cursor: &tree_sitter::TreeCursor, output: &mut String) {
    if cursor.node().kind() == ":" {
        output.push(' ');
    }
}

fn adapt_indent_level(cursor: &tree_sitter::TreeCursor, indent_level: &mut usize) {
    match cursor.node().kind() {
        "(" => {
            *indent_level += 2;
        }
        ")" => {
            *indent_level -= 2;
        }
        "[" => {
            *indent_level += 1;
        }
        "]" => {
            *indent_level -= 1;
        }
        _ => {}
    }
}

fn indent_list_contents(
    cursor: &tree_sitter::TreeCursor,
    output: &mut String,
    indent_level: usize,
) {
    if (cursor.node().kind() == "anonymous_node" || cursor.node().kind() == "named_node")
        && check_parent("list", cursor.node())
    {
        output.push('\n');
        output.push_str(&" ".repeat(indent_level));
    }
}
