use scraper::{Html, Selector};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with default syntax definitions and themes
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight all code blocks in the HTML string
    pub fn highlight_html(&self, html: &str) -> String {
        self.highlight_html_with_theme(html, "base16-ocean.dark")
    }

    /// Highlight all code blocks in the HTML string with a specific theme
    pub fn highlight_html_with_theme(&self, html: &str, theme_name: &str) -> String {
        let document = Html::parse_document(html);
        let pre_selector = Selector::parse("pre").unwrap();
        let code_selector = Selector::parse("code").unwrap();

        let theme = &self.theme_set.themes[theme_name];

        // Collect all pre>code blocks with their information
        let mut replacements = Vec::new();

        for pre_element in document.select(&pre_selector) {
            // Check if this pre has a direct code child
            if let Some(code_element) = pre_element.select(&code_selector).next() {
                // Get the plain text content (strips any existing HTML tags/spans)
                let code_text = code_element.text().collect::<String>();

                // Skip empty code blocks
                if code_text.trim().is_empty() {
                    continue;
                }

                // DEBUG: Print what we're working with
                // println!("=== CODE BLOCK DEBUG ===");
                // println!("Language: {:?}", self.detect_language(&code_element));
                // println!("Code length: {} chars", code_text.len());
                // println!("Newline count: {}", code_text.matches('\n').count());
                // println!("First 200 chars: {:?}", &code_text[..code_text.len().min(200)]);

                let language = self.detect_language(&code_element);
                let highlighted = self.highlight_code(&code_text, &language, theme);

                // println!("Highlighted length: {} chars", highlighted.len());
                // println!("Span count: {}", highlighted.matches("<span").count());
                // println!("First 200 chars of output: {:?}", &highlighted[..highlighted.len().min(200)]);
                // println!("========================\n");

                // Get original HTML fragments
                let pre_html = pre_element.html();

                // Build the replacement with proper attributes
                let mut new_code = String::from("<code");
                for (name, value) in code_element.value().attrs() {
                    new_code.push_str(&format!(r#" {}="{}""#, name, value));
                }
                new_code.push('>');
                new_code.push_str(&highlighted);
                new_code.push_str("</code>");

                replacements.push((pre_html.clone(), new_code));
            }
        }

        // Apply replacements
        let mut result = html.to_string();
        for (old_pre, new_code) in replacements {
            // Find and replace the entire <pre> block
            if let Some(start) = result.find(&old_pre) {
                // Build the new pre tag
                let new_pre = if let Some(code_pos) = old_pre.find("<code") {
                    let pre_opening = &old_pre[..code_pos];
                    let closing = "</pre>";
                    format!("{}{}{}", pre_opening, new_code, closing)
                } else {
                    continue;
                };

                result = result.replacen(&old_pre, &new_pre, 1);
            }
        }

        result
    }

    /// Detect the programming language from the code element's class attribute
    fn detect_language(&self, element: &scraper::ElementRef) -> String {
        let detected = element
            .value()
            .attr("class")
            .and_then(|class| {
                // Look for patterns like "language-rust" or "lang-python"
                class.split_whitespace().find_map(|c| {
                    c.strip_prefix("language-")
                        .or_else(|| c.strip_prefix("lang-"))
                })
            })
            .unwrap_or("txt")
            .to_string();

        // Apply fallbacks for languages that might not be in syntect defaults
        match detected.as_str() {
            // TypeScript → JavaScript (syntect's default bundle often doesn't have TS)
            "typescript" | "ts" | "tsx" => {
                // Check if TypeScript is available, otherwise use JavaScript
                if self.syntax_set.find_syntax_by_extension("ts").is_some() {
                    detected
                } else {
                    println!("TypeScript syntax not found, falling back to JavaScript");
                    "js".to_string()
                }
            }
            // JSX → JavaScript
            "jsx" => "js".to_string(),
            _ => detected,
        }
    }

    /// Highlight a code snippet with the specified language and theme
    fn highlight_code(
        &self,
        code: &str,
        language: &str,
        theme: &syntect::highlighting::Theme,
    ) -> String {
        // Find the syntax definition for the language
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(language)
            .or_else(|| self.syntax_set.find_syntax_by_name(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        println!("Using syntax: {}", syntax.name);

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut highlighted_html = String::new();

        // Process line by line
        for (i, line) in LinesWithEndings::from(code).enumerate() {
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            if i == 0 {
                println!("First line has {} tokens", ranges.len());
            }

            // Convert to HTML spans (without background color)
            let html = styled_line_to_highlighted_html(&ranges[..], IncludeBackground::No)
                .unwrap_or_else(|_| line.to_string());

            highlighted_html.push_str(&html);
        }

        highlighted_html
    }

    /// Get list of available themes
    pub fn available_themes(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }

    /// Get list of available languages
    pub fn available_languages(&self) -> Vec<&str> {
        self.syntax_set
            .syntaxes()
            .iter()
            .map(|s| s.name.as_str())
            .collect()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_highlighting() {
        let highlighter = SyntaxHighlighter::new();
        let html = r#"<pre><code class="language-rust">fn main() {
    println!("Hello, world!");
}</code></pre>"#;

        let result = highlighter.highlight_html(html);
        assert!(result.contains("<span"));
        assert!(result.contains("fn"));
    }

    #[test]
    fn test_multiple_code_blocks() {
        let highlighter = SyntaxHighlighter::new();
        let html = r#"
<pre><code class="language-rust">fn test() {}</code></pre>
<pre><code class="language-python">def test(): pass</code></pre>
"#;

        let result = highlighter.highlight_html(html);
        assert!(result.contains("<span"));
    }

    #[test]
    fn test_no_language_specified() {
        let highlighter = SyntaxHighlighter::new();
        let html = r#"<pre><code>Some plain text</code></pre>"#;

        let result = highlighter.highlight_html(html);
        // Should still process without errors
        assert!(result.contains("Some plain text"));
    }

    #[test]
    fn test_typescript_highlighting() {
        let highlighter = SyntaxHighlighter::new();
        // Properly formatted TypeScript
        let html = r#"<pre><code class="language-typescript">import {Component, OnInit} from '@angular/core';

@Component({
  selector: 'app-my-form'
})
export class MyFormComponent implements OnInit {
  constructor(private fb: FormBuilder) {}
}</code></pre>"#;

        let result = highlighter.highlight_html(html);

        // Should have highlighting spans
        assert!(result.contains("<span"));

        // Verify key keywords appear in the output
        assert!(result.contains("import"));
        assert!(result.contains("Component"));
        assert!(result.contains("class"));
    }

    #[test]
    fn test_strips_existing_spans() {
        let highlighter = SyntaxHighlighter::new();
        // Code with existing span tags (from previous highlighting or editor)
        let html = r#"<pre><code class="language-rust"><span style="color:#c0c5ce;">fn main() { println!("test"); }</span></code></pre>"#;

        let result = highlighter.highlight_html(html);

        // Old color should be gone
        assert!(!result.contains("color:#c0c5ce;\">[entire line]"));

        // Should have new highlighting
        assert!(result.contains("<span"));
        assert!(result.contains("fn"));
    }

    #[test]
    fn test_typescript_falls_back_to_javascript() {
        let highlighter = SyntaxHighlighter::new();
        // TypeScript code
        let html = r#"<pre><code class="language-typescript">const x: number = 5;
function test(): void {
    console.log(x);
}</code></pre>"#;

        let result = highlighter.highlight_html(html);

        // Should have highlighting (even if falling back to JS syntax)
        assert!(result.contains("<span"));
        assert!(result.contains("const"));
        assert!(result.contains("function"));
    }
}
