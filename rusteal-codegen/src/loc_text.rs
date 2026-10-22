//! The `loctext!` calls of Rust sources, written as `NSLOCTEXT` lines of a C++ header that
//! Unreal's localization gather (`GatherTextFromSource`) reads; nothing compiles the header.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::{LitStr, Token};

pub const HEADER: &str = "RustealText.h";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocText {
    pub namespace: String,
    pub key: String,
    pub source: String,
}

pub fn texts_in(source: &str) -> Vec<LocText> {
    let Ok(tokens) = TokenStream::from_str(source) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    collect(tokens, &mut found);
    found
}

fn collect(tokens: TokenStream, found: &mut Vec<LocText>) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();

    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            let is_call = index >= 2
                && matches!(&tokens[index - 1], TokenTree::Punct(p) if p.as_char() == '!')
                && matches!(&tokens[index - 2], TokenTree::Ident(i) if i == "loctext");

            if is_call && let Some(text) = parse_call(group.stream()) {
                found.push(text);
                continue;
            }

            collect(group.stream(), found);
        }
    }
}

fn parse_call(args: TokenStream) -> Option<LocText> {
    let parsed =
        syn::parse::Parser::parse2(Punctuated::<LitStr, Token![,]>::parse_terminated, args).ok()?;
    let parts: Vec<String> = parsed.iter().map(LitStr::value).collect();

    match parts.as_slice() {
        [namespace, key, source] => Some(LocText {
            namespace: namespace.clone(),
            key: key.clone(),
            source: source.clone(),
        }),
        _ => None,
    }
}

pub fn texts_in_dirs(dirs: &[PathBuf]) -> Vec<LocText> {
    let mut found = Vec::new();

    for dir in dirs {
        walk(dir, &mut found);
    }

    found.sort();
    found.dedup();
    found
}

fn walk(dir: &Path, found: &mut Vec<LocText>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            found.extend(texts_in(&text));
        }
    }
}

fn cpp_string(value: &str) -> String {
    let mut out = String::from("\"");

    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }

    out.push('"');
    out
}

pub fn header_text(texts: &[LocText]) -> String {
    let mut out = String::from(
        "// Written by rusteal build from the loctext! calls of the project's Rust crates, for\n\
         // Unreal's localization gather. Nothing includes it.\n\n#pragma once\n\n",
    );

    for text in texts {
        out.push_str(&format!(
            "NSLOCTEXT({}, {}, {})\n",
            cpp_string(&text.namespace),
            cpp_string(&text.key),
            cpp_string(&text.source)
        ));
    }

    out
}

/// Writes the header at `path` for `texts`, or removes it when there are none; returns whether
/// the file changed.
pub fn write_header(path: &Path, texts: &[LocText]) -> std::io::Result<bool> {
    if texts.is_empty() {
        return Ok(std::fs::remove_file(path).is_ok());
    }

    let text = header_text(texts);

    if std::fs::read_to_string(path).ok().as_deref() == Some(text.as_str()) {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, text)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loctext_calls_are_found_anywhere() {
        let source = r#"
            fn title() -> Text {
                loctext!("Menu", "Title", "Main \"menu\"")
            }

            // loctext!("Commented", "Out", "Not gathered")

            fn nested() {
                log::info!("{}", loctext!("Hud", "Ammo", "Ammo: {0}"));
                let not_text = other!("Menu", "Title", "x");
                let short = loctext!("Only", "Two");
            }
        "#;

        let texts = texts_in(source);

        assert_eq!(
            texts,
            [
                LocText {
                    namespace: "Menu".into(),
                    key: "Title".into(),
                    source: "Main \"menu\"".into(),
                },
                LocText {
                    namespace: "Hud".into(),
                    key: "Ammo".into(),
                    source: "Ammo: {0}".into(),
                },
            ]
        );

        assert_eq!(
            header_text(&texts[..1]).lines().last(),
            Some(r#"NSLOCTEXT("Menu", "Title", "Main \"menu\"")"#)
        );
    }
}
