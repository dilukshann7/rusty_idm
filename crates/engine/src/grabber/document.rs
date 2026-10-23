//! Shared discovery/export parsers: export never scans rewritten documents.
use super::ItemKind;
use cssparser::{Parser, Token};
use lol_html::{element, rewrite_str, text, RewriteStrSettings};
use std::cell::RefCell;
use url::Url;

#[derive(Clone)]
pub(crate) struct Link {
    pub url: String,
    pub navigation: bool,
    pub kind: ItemKind,
}

pub(crate) fn decode(bytes: &[u8], content_type: &str) -> String {
    let label = content_type.split(';').find_map(|part| {
        part.trim()
            .split_once('=')
            .filter(|(k, _)| k.eq_ignore_ascii_case("charset"))
            .map(|(_, v)| v.trim().trim_matches(['\'', '"']).to_owned())
    });
    let mut label = label.or_else(|| {
        std::str::from_utf8(&bytes[..bytes.len().min(1024)])
            .ok()
            .and_then(|s| s.strip_prefix("@charset \""))
            .and_then(|s| s.split('"').next())
            .map(str::to_owned)
    });
    if label.is_none() {
        let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
        let _ = rewrite_str(
            &prefix,
            RewriteStrSettings::new().append_element_content_handler(element!("meta", |el| {
                if label.is_none() {
                    label = el.get_attribute("charset").or_else(|| {
                        el.get_attribute("content").and_then(|v| {
                            v.split(';').find_map(|p| {
                                p.trim()
                                    .split_once('=')
                                    .filter(|(k, _)| k.eq_ignore_ascii_case("charset"))
                                    .map(|(_, v)| v.trim().to_owned())
                            })
                        })
                    });
                }
                Ok(())
            })),
        );
    }
    let encoding = label
        .as_deref()
        .and_then(|l| encoding_rs::Encoding::for_label(l.as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    encoding.decode(bytes).0.into_owned()
}

pub(crate) enum DiscoveryError {
    Limit,
    Parse(String),
}
impl From<String> for DiscoveryError {
    fn from(value: String) -> Self {
        Self::Parse(value)
    }
}
pub(crate) fn discover(
    input: &str,
    base: &Url,
    kind: ItemKind,
    limit: usize,
) -> Result<Vec<Link>, DiscoveryError> {
    let mut links = vec![];
    let mut visit = |value: &str, navigation: bool, kind: ItemKind| {
        if let Ok(url) = base.join(value.trim()) {
            if matches!(url.scheme(), "http" | "https") && links.len() <= limit {
                links.push(Link {
                    url: url.into(),
                    navigation,
                    kind,
                });
            }
        }
        value.to_owned()
    };
    if kind == ItemKind::Stylesheet {
        css(input, &mut |v, k| visit(v, false, k), 0);
    } else {
        html(input, base, &mut visit)?;
    }
    if links.len() > limit {
        return Err(DiscoveryError::Limit);
    }
    Ok(links)
}

pub(crate) fn html(
    input: &str,
    base: &Url,
    visit: &mut dyn FnMut(&str, bool, ItemKind) -> String,
) -> Result<String, String> {
    let mut effective = base.clone();
    let mut found = false;
    rewrite_str(
        input,
        RewriteStrSettings::new().append_element_content_handler(element!("base[href]", |el| {
            if !found {
                if let Some(value) = el.get_attribute("href") {
                    if let Ok(url) = base.join(&value) {
                        effective = url;
                        found = true;
                    }
                }
            }
            Ok(())
        })),
    )
    .map_err(|_| "Cannot parse HTML base URL.")?;
    let visitor = RefCell::new(visit);
    let resolve = |value: &str, nav: bool, kind: ItemKind| -> String {
        if value.trim().starts_with('#') {
            return value.into();
        }
        match effective.join(value.trim()) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => {
                (visitor.borrow_mut())(url.as_str(), nav, kind)
            }
            _ => value.into(),
        }
    };
    let mut style = String::new();
    let body = rewrite_str(
        input,
        RewriteStrSettings::new()
            .append_element_content_handler(element!("*", |el| {
                let tag = el.tag_name();
                if tag == "base" {
                    el.remove();
                    return Ok(());
                }
                if tag == "meta" {
                    if el.get_attribute("charset").is_some() {
                        el.set_attribute("charset", "utf-8")?;
                    }
                    if el
                        .get_attribute("http-equiv")
                        .is_some_and(|v| v.eq_ignore_ascii_case("content-type"))
                    {
                        el.set_attribute("content", "text/html; charset=utf-8")?;
                    }
                }
                for attr in [
                    "href",
                    "src",
                    "poster",
                    "data",
                    "action",
                    "formaction",
                    "cite",
                    "background",
                    "longdesc",
                ] {
                    if let Some(value) = el.get_attribute(attr) {
                        let navigation = (attr == "href" && matches!(tag.as_str(), "a" | "area"))
                            || (attr == "src" && matches!(tag.as_str(), "iframe" | "frame"));
                        let kind = if tag == "link"
                            && el.get_attribute("rel").is_some_and(|v| {
                                v.split_whitespace()
                                    .any(|p| p.eq_ignore_ascii_case("stylesheet"))
                            }) {
                            ItemKind::Stylesheet
                        } else {
                            ItemKind::Unknown
                        };
                        // Form submissions are absolutized but never discovered/fetched.
                        let updated =
                            if matches!(attr, "action" | "formaction" | "cite" | "longdesc") {
                                effective
                                    .join(&value)
                                    .map(|u| u.to_string())
                                    .unwrap_or(value)
                            } else {
                                resolve(&value, navigation, kind)
                            };
                        el.set_attribute(attr, &updated)?;
                    }
                }
                for attr in ["srcset", "imagesrcset"] {
                    if let Some(value) = el.get_attribute(attr) {
                        el.set_attribute(
                            attr,
                            &srcset(&value, &mut |v| resolve(v, false, ItemKind::Unknown)),
                        )?;
                    }
                }
                if let Some(value) = el.get_attribute("style") {
                    el.set_attribute("style", &css(&value, &mut |v, k| resolve(v, false, k), 0))?;
                }
                Ok(())
            }))
            .append_element_content_handler(text!("style", |chunk| {
                style.push_str(chunk.as_str());
                if chunk.last_in_text_node() {
                    let rewritten = css(&style, &mut |v, k| resolve(v, false, k), 0);
                    chunk.replace(&rewritten, lol_html::html_content::ContentType::Html);
                    style.clear();
                } else {
                    chunk.remove();
                }
                Ok(())
            })),
    )
    .map_err(|_| "Cannot parse or rewrite HTML.")?;
    // UTF-8 BOM declares encoding without moving the doctype into quirks mode.
    Ok(format!("\u{feff}{body}"))
}

pub(crate) fn css(
    input: &str,
    visit: &mut dyn FnMut(&str, ItemKind) -> String,
    depth: usize,
) -> String {
    if depth >= 64 {
        return input.into();
    }
    let mut parser = Parser::new(input);
    css_tokens(&mut parser, visit, depth)
}
fn quoted(value: &str) -> String {
    let mut out = String::new();
    let _ = cssparser::serialize_string(value, &mut out);
    out
}
fn css_tokens(
    parser: &mut Parser<'_>,
    visit: &mut dyn FnMut(&str, ItemKind) -> String,
    depth: usize,
) -> String {
    let mut output = String::new();
    let mut import = false;
    let mut charset = false;
    loop {
        let start = parser.position();
        let Ok(token) = parser.next_including_whitespace_and_comments().cloned() else {
            break;
        };
        let raw = parser.slice_from(start).to_owned();
        match token {
            Token::AtKeyword(name) => {
                import = name.eq_ignore_ascii_case("import");
                charset = name.eq_ignore_ascii_case("charset");
                output.push_str(&raw);
            }
            Token::UnquotedUrl(value) => {
                output.push_str("url(");
                output.push_str(&quoted(&visit(
                    &value,
                    if import {
                        ItemKind::Stylesheet
                    } else {
                        ItemKind::Asset
                    },
                )));
                output.push(')');
                import = false;
            }
            Token::QuotedString(value) if import || charset => {
                output.push_str(&quoted(&if charset {
                    "utf-8".into()
                } else {
                    visit(&value, ItemKind::Stylesheet)
                }));
                import = false;
                charset = false;
            }
            Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                let value = parser.parse_nested_block(|p| {
                    p.expect_string_cloned()
                        .map(|v| v.to_string())
                        .map_err(cssparser::ParseError::<()>::from)
                });
                match value {
                    Ok(value) => {
                        output.push_str("url(");
                        output.push_str(&quoted(&visit(
                            &value,
                            if import {
                                ItemKind::Stylesheet
                            } else {
                                ItemKind::Asset
                            },
                        )));
                        output.push(')');
                    }
                    Err(_) => output.push_str(parser.slice_from(start)),
                };
                import = false;
            }
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock
                if depth < 64 =>
            {
                let close = match token {
                    Token::SquareBracketBlock => ']',
                    Token::CurlyBracketBlock => '}',
                    _ => ')',
                };
                let nested: Result<String, cssparser::ParseError<()>> =
                    parser.parse_nested_block(|p| Ok(css_tokens(p, visit, depth + 1)));
                output.push_str(&raw);
                if let Ok(nested) = nested {
                    output.push_str(&nested);
                }
                output.push(close);
            }
            Token::Semicolon => {
                import = false;
                charset = false;
                output.push_str(&raw);
            }
            _ => output.push_str(&raw),
        }
    }
    output
}

fn srcset(input: &str, visit: &mut dyn FnMut(&str) -> String) -> String {
    let mut rest = input;
    let mut result = vec![];
    while !rest
        .trim_matches(|c: char| c.is_ascii_whitespace() || c == ',')
        .is_empty()
    {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        let end = rest
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        let token = &rest[..end];
        rest = &rest[end..];
        let url = token.trim_end_matches(',');
        let mut candidate = visit(url);
        if !token.ends_with(',') {
            let mut parens = 0;
            let mut end = rest.len();
            for (i, c) in rest.char_indices() {
                match c {
                    '(' => parens += 1,
                    ')' => parens -= 1,
                    ',' if parens == 0 => {
                        end = i;
                        break;
                    }
                    _ => {}
                }
            }
            let descriptor = rest[..end].trim();
            if !descriptor.is_empty() {
                candidate.push(' ');
                candidate.push_str(descriptor);
            }
            rest = &rest[end..];
        }
        result.push(candidate);
    }
    result.join(", ")
}
