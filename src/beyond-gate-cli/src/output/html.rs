//! Bounded local conversion of an already checked selected HTML subtree.

use scraper::{ElementRef, Html};

use super::{Failure, append, escape_cell, markdown_text};

/// Retains block boundaries, headings, lists, tables, and links without further reads.
pub(super) fn convert(content: &str, markdown: bool) -> Result<String, Failure> {
    if content.len() > 1024 * 1024 {
        return Err(Failure::RecordPresentationLimit);
    }
    let document = Html::parse_fragment(content);
    validate(&document)?;
    let mut stack = vec![(document.tree.root(), false, 0_usize)];
    let mut result = String::new();
    let mut lists: Vec<Option<(i64, i64)>> = Vec::new();
    let mut tables: Vec<(bool, usize)> = Vec::new();
    let mut count = 0_usize;
    while let Some((node, closing, depth)) = stack.pop() {
        if depth > 256 || count > 100_000 {
            return Err(Failure::RecordPresentationLimit);
        }
        if !closing {
            count += 1;
        }
        let Some(element) = ElementRef::wrap(node) else {
            if let Some(text) = node.value().as_text() {
                words(&mut result, text, markdown)?;
            }
            for child in node.children().rev() {
                stack.push((child, false, depth + 1));
            }
            continue;
        };
        let name = element.value().name();
        if matches!(
            name,
            "script"
                | "style"
                | "form"
                | "input"
                | "button"
                | "iframe"
                | "object"
                | "embed"
                | "textarea"
                | "select"
        ) {
            return Err(Failure::InvalidHtml);
        }
        if closing {
            match name {
                "ul" | "ol" => {
                    lists.pop();
                }
                "a" => {
                    if let Some(href) = element.attr("href") {
                        let href = destination(href)?;
                        append(
                            &mut result,
                            &if markdown {
                                format!("]({href})")
                            } else {
                                format!(" ({href})")
                            },
                        )?;
                    }
                }
                "strong" | "b" if markdown => append(&mut result, "**")?,
                "em" | "i" if markdown => append(&mut result, "*")?,
                "td" | "th" => append(&mut result, if markdown { " | " } else { "\t" })?,
                "tr" => {
                    newline(&mut result, false)?;
                    if markdown && let Some((first, columns)) = tables.last_mut() {
                        if *columns == 0 {
                            return Err(Failure::InvalidHtml);
                        }
                        if *first {
                            append(&mut result, "|")?;
                            for _ in 0..*columns {
                                append(&mut result, " --- |")?;
                            }
                            newline(&mut result, false)?;
                            *first = false;
                        }
                    }
                }
                "table" => {
                    tables.pop();
                }
                _ => {}
            }
            if block(name) && tables.is_empty() {
                newline(&mut result, true)?;
            }
            continue;
        }
        if matches!(name, "pre" | "code") {
            let text = element.text().collect::<String>();
            let text = safe_preformatted(&text)?;
            if markdown {
                let length = text
                    .split(|value| value != '`')
                    .map(str::len)
                    .max()
                    .unwrap_or(0);
                let fence = "`".repeat((length + 1).max(if name == "pre" { 3 } else { 1 }));
                if name == "pre" {
                    newline(&mut result, true)?;
                    append(&mut result, &format!("{fence}\n{text}\n{fence}\n"))?;
                } else {
                    append(&mut result, &format!("{fence} {text} {fence}"))?;
                }
            } else {
                append(&mut result, &text)?;
            }
            continue;
        }
        if block(name) && tables.is_empty() {
            newline(&mut result, true)?;
        }
        match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if markdown => {
                let rank = usize::from(name.as_bytes()[1] - b'0');
                append(&mut result, &format!("{} ", "#".repeat(rank)))?;
            }
            "ul" => lists.push(None),
            "ol" => {
                let reversed = element.attr("reversed").is_some();
                let count = element
                    .children()
                    .filter_map(ElementRef::wrap)
                    .filter(|child| child.value().name() == "li")
                    .count() as i64;
                lists.push(Some((
                    number(element.attr("start"))?.unwrap_or(if reversed { count } else { 1 }),
                    if reversed { -1 } else { 1 },
                )));
            }
            "li" => {
                newline(&mut result, false)?;
                append(&mut result, &"  ".repeat(lists.len().saturating_sub(1)))?;
                let marker = if let Some(Some((next, step))) = lists.last_mut() {
                    let value = number(element.attr("value"))?.unwrap_or(*next);
                    *next = value.checked_add(*step).ok_or(Failure::InvalidHtml)?;
                    format!("{value}. ")
                } else {
                    "- ".to_owned()
                };
                append(&mut result, &marker)?;
            }
            "br" | "hr" => {
                if tables.is_empty() {
                    newline(&mut result, false)?;
                } else {
                    append(&mut result, " ")?;
                }
            }
            "a" if element.attr("href").is_some() && markdown => append(&mut result, "[")?,
            "strong" | "b" if markdown => append(&mut result, "**")?,
            "em" | "i" if markdown => append(&mut result, "*")?,
            "img" => {
                if let Some(alt) = element.attr("alt") {
                    words(&mut result, alt, markdown)?;
                }
            }
            "table" => tables.push((true, 0)),
            "tr" => {
                newline(&mut result, false)?;
                if let Some((_, columns)) = tables.last_mut() {
                    *columns = 0;
                }
                if markdown {
                    append(&mut result, "| ")?;
                }
            }
            "td" | "th" => {
                if element.attr("colspan").is_some_and(|value| value != "1")
                    || element.attr("rowspan").is_some_and(|value| value != "1")
                {
                    return Err(Failure::InvalidHtml);
                }
                if let Some((_, columns)) = tables.last_mut() {
                    *columns += 1;
                }
            }
            _ => {}
        }
        stack.push((node, true, depth));
        for child in node.children().rev() {
            stack.push((child, false, depth + 1));
        }
    }
    Ok(result.trim().to_owned())
}

/// Checks every DOM node, including subtrees later emitted as code.
fn validate(document: &Html) -> Result<(), Failure> {
    let mut stack = vec![(document.tree.root(), 0_usize)];
    let mut count = 0_usize;
    while let Some((node, depth)) = stack.pop() {
        count += 1;
        if depth > 256 || count > 100_000 {
            return Err(Failure::RecordPresentationLimit);
        }
        if let Some(element) = ElementRef::wrap(node)
            && matches!(
                element.value().name(),
                "script"
                    | "style"
                    | "form"
                    | "input"
                    | "button"
                    | "iframe"
                    | "object"
                    | "embed"
                    | "textarea"
                    | "select"
            )
        {
            return Err(Failure::InvalidHtml);
        }
        for child in node.children().rev() {
            stack.push((child, depth + 1));
        }
    }
    Ok(())
}

/// Normalizes source indentation while retaining boundaries between text nodes.
fn words(result: &mut String, text: &str, markdown: bool) -> Result<(), Failure> {
    let mut space = false;
    for character in text.chars() {
        if character.is_whitespace() {
            space = true;
        } else {
            if space && !result.is_empty() && !result.ends_with([' ', '\n', '\t']) {
                append(result, " ")?;
            }
            let text = escape_cell(&character.to_string());
            append(result, &if markdown { markdown_text(&text) } else { text })?;
            space = false;
        }
    }
    if space && !result.ends_with([' ', '\n', '\t']) {
        append(result, " ")?;
    }
    Ok(())
}

/// Separates blocks without whitespace-only lines.
fn newline(result: &mut String, blank: bool) -> Result<(), Failure> {
    while result.ends_with([' ', '\t']) {
        result.pop();
    }
    if !result.is_empty() && !result.ends_with('\n') {
        append(result, "\n")?;
    }
    if blank && !result.is_empty() && !result.ends_with("\n\n") {
        append(result, "\n")?;
    }
    Ok(())
}

/// Recognizes structural blocks independently of CSS or browser execution.
fn block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "section"
            | "article"
            | "blockquote"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
            | "table"
    )
}

/// Keeps link destinations as inert text and rejects active or malformed schemes.
fn destination(href: &str) -> Result<String, Failure> {
    if href.chars().any(char::is_control)
        || href.starts_with("//")
        || !(href.starts_with("https://")
            || href.starts_with("http://")
            || href.starts_with('/')
            || href.starts_with('#'))
    {
        return Err(Failure::InvalidHtml);
    }
    Ok(href
        .replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace('\\', "%5C"))
}

/// Preserves code line breaks while preventing terminal controls.
fn safe_preformatted(text: &str) -> Result<String, Failure> {
    if text
        .chars()
        .any(|value| value.is_control() && !matches!(value, '\n' | '\r' | '\t'))
    {
        return Err(Failure::InvalidHtml);
    }
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Checks explicit list counters rather than silently changing numbering.
fn number(text: Option<&str>) -> Result<Option<i64>, Failure> {
    text.map(|text| {
        text.parse::<i32>()
            .map(i64::from)
            .map_err(|_| Failure::InvalidHtml)
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves selected headings, numbered lists, tables, and inert link destinations.
    #[test]
    fn converts_structured_html_without_executing_or_fetching_it() {
        let html = "<h2>Invented heading</h2><p>A <strong>bright</strong> lantern.</p>\
                    <ol start=\"3\"><li>First</li><li value=\"8\">Second</li></ol>\
                    <table><tr><th>Name</th><th>Count</th></tr><tr><td>Lantern</td><td>2</td></tr></table>\
                    <p><a href=\"/sources/invented#light\">Source</a></p>";
        let markdown = convert(html, true).expect("Markdown");
        for expected in [
            "## Invented heading",
            "**bright**",
            "3. First",
            "8. Second",
            "| Name | Count |",
            "| --- | --- |",
            "| Lantern | 2 |",
            "[Source](/sources/invented#light)",
        ] {
            assert!(markdown.contains(expected), "{expected}: {markdown}");
        }
        let text = convert(html, false).expect("text");
        assert!(text.contains("Invented heading"));
        assert!(text.contains("Name\tCount"));
        assert!(text.contains("Source (/sources/invented#light)"));
        assert!(!text.contains("<table"));
    }

    /// Keeps code fences intact while escaping source markup and rejecting unsafe constructs.
    #[test]
    fn conversion_rejects_unsafe_html_and_handles_literal_code() {
        for html in [
            "<script>bad</script>",
            "<form>bad</form>",
            "<a href=\"javascript:bad\">bad</a>",
            "<pre><iframe></iframe></pre>",
        ] {
            assert!(matches!(convert(html, true), Err(Failure::InvalidHtml)));
        }
        let markdown = convert(
            "<p>&lt;script&gt;*literal*</p><pre>``` code\nnext</pre>",
            true,
        )
        .expect("inert Markdown");
        assert!(markdown.contains("&lt;script&gt;\\*literal\\*"));
        assert!(markdown.contains("````\n``` code\nnext\n````"));
        assert!(matches!(
            convert(
                &format!("{}x{}", "<div>".repeat(257), "</div>".repeat(257)),
                true
            ),
            Err(Failure::RecordPresentationLimit)
        ));
    }
}
