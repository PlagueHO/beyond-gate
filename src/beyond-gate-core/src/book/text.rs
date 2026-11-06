//! Iterative plain-text conversion for one explicitly selected heading range.

use scraper::ElementRef;

use super::BookError;

/// Returns the HTML heading rank without inferring hierarchy from CSS.
pub(super) fn heading(element: ElementRef<'_>) -> Option<u8> {
    match element.value().name() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// Tracks deterministic whitespace, preformatted text, and nested list counters.
#[derive(Default)]
struct Text {
    output: String,
    space: bool,
    pre: usize,
    lists: Vec<Option<(i64, i64)>>,
}

impl Text {
    /// Preserves word separation without source indentation or non-text controls.
    fn append(&mut self, value: &str) -> Result<(), BookError> {
        for value in value.chars() {
            if value.is_control() && !matches!(value, '\n' | '\r' | '\t') {
                return Err(BookError::MalformedSource);
            }
            if self.pre != 0 {
                if value != '\r' {
                    self.output.push(value);
                }
            } else if value.is_whitespace() {
                self.space = true;
            } else {
                if self.space
                    && !self.output.is_empty()
                    && !self.output.ends_with([' ', '\n', '\t'])
                {
                    self.output.push(' ');
                }
                self.output.push(value);
                self.space = false;
            }
        }
        if self.output.len() > 1024 * 1024 {
            return Err(BookError::LimitExceeded);
        }
        Ok(())
    }

    /// Separates blocks without creating repeated blank lines or trailing spaces.
    fn newline(&mut self) {
        while self.output.ends_with([' ', '\t']) {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.space = false;
    }

    /// Starts a real ordered or unordered list, including explicit reverse numbering.
    fn list(&mut self, element: ElementRef<'_>) -> Result<(), BookError> {
        let next = if element.value().name() == "ol" {
            let reverse = element.attr("reversed").is_some();
            let count = element
                .children()
                .filter_map(ElementRef::wrap)
                .filter(|child| child.value().name() == "li")
                .count() as i64;
            let next = integer(element.attr("start"))?.unwrap_or(if reverse { count } else { 1 });
            Some((next, if reverse { -1 } else { 1 }))
        } else {
            None
        };
        self.lists.push(next);
        Ok(())
    }

    /// Preserves list numbering and item overrides rather than flattening every list.
    fn item(&mut self, element: ElementRef<'_>) -> Result<(), BookError> {
        self.newline();
        if let Some(Some((next, step))) = self.lists.last_mut() {
            let value = integer(element.attr("value"))?.unwrap_or(*next);
            *next = value.checked_add(*step).ok_or(BookError::MalformedSource)?;
            self.output.push_str(&format!("{value}. "));
        } else {
            self.output.push_str("- ");
        }
        Ok(())
    }
}

/// Rejects invalid explicit list numbering rather than changing the author's order.
fn integer(value: Option<&str>) -> Result<Option<i64>, BookError> {
    value
        .map(|value| {
            value
                .parse::<i32>()
                .map(i64::from)
                .map_err(|_| BookError::MalformedSource)
        })
        .transpose()
}

/// Identifies text blocks whose opening and closing edges need line separation.
fn block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "section"
            | "article"
            | "aside"
            | "blockquote"
            | "pre"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "table"
            | "tr"
            | "caption"
            | "figure"
            | "figcaption"
    )
}

/// Excludes executable, navigation, control, and hidden subtrees, but preserves asides.
fn excluded(element: ElementRef<'_>) -> bool {
    matches!(
        element.value().name(),
        "script"
            | "style"
            | "nav"
            | "form"
            | "button"
            | "input"
            | "textarea"
            | "select"
            | "iframe"
            | "object"
            | "embed"
            | "canvas"
            | "svg"
            | "template"
            | "noscript"
    ) || element.attr("hidden").is_some()
        || element
            .attr("aria-hidden")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        || element.attr("style").is_some_and(|style| {
            let style: String = style
                .chars()
                .filter(|value| !value.is_whitespace())
                .collect();
            let style = style.to_ascii_lowercase();
            style.contains("display:none") || style.contains("visibility:hidden")
        })
}

/// Walks bounded DOM edges without recursive calls or adjacent-section text leakage.
pub(super) fn render(
    article: ElementRef<'_>,
    selected: Option<ElementRef<'_>>,
) -> Result<String, BookError> {
    let mut stack = vec![(*article, false, 0_usize)];
    let mut text = Text::default();
    let mut active = selected.is_none();
    let rank = selected.and_then(heading);
    let mut count = 0;
    while let Some((node, closing, depth)) = stack.pop() {
        if depth > 256 || text.output.len() > 1024 * 1024 {
            return Err(BookError::LimitExceeded);
        }
        let Some(element) = ElementRef::wrap(node) else {
            if active && let Some(value) = node.value().as_text() {
                text.append(value)?;
            }
            continue;
        };
        let name = element.value().name();
        if closing {
            if matches!(name, "ol" | "ul") {
                text.lists.pop();
            }
            if name == "pre" {
                text.pre -= 1;
            }
            if active && block(name) {
                text.newline();
            }
            if active && matches!(name, "td" | "th") {
                text.space = false;
                text.output.push('\t');
            }
            continue;
        }
        count += 1;
        if count > 100_000 {
            return Err(BookError::LimitExceeded);
        }
        if excluded(element) {
            continue;
        }
        let starts = selected.is_some_and(|selected| selected.id() == element.id());
        if starts {
            active = true;
        } else if active
            && let (Some(level), Some(rank)) = (heading(element), rank)
            && level <= rank
        {
            break;
        }
        if matches!(name, "ol" | "ul") {
            text.list(element)?;
        }
        if active {
            if block(name) {
                text.newline();
            }
            match name {
                "li" => text.item(element)?,
                "br" | "hr" => text.newline(),
                "img" => {
                    if let Some(alt) = element.attr("alt") {
                        text.append(alt)?;
                    }
                }
                _ => {}
            }
        }
        if name == "pre" {
            text.pre += 1;
        }
        stack.push((node, true, depth));
        for child in node.children().rev() {
            stack.push((child, false, depth + 1));
        }
    }
    if !active {
        return Err(BookError::UnsupportedSection);
    }
    text.newline();
    if text.output.len() > 1024 * 1024 {
        return Err(BookError::LimitExceeded);
    }
    Ok(text.output)
}
