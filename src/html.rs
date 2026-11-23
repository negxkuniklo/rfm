use crate::ast::{Alignment, Block, Document, FootnoteDef, Inline, ListItem};

pub fn render_document(doc: &Document) -> String {
    let mut out = String::new();
    render_blocks(&doc.blocks, &mut out);
    if !doc.footnotes.is_empty() {
        render_footnotes(&doc.footnotes, &mut out);
    }
    out
}

fn render_blocks(blocks: &[Block], out: &mut String) {
    for block in blocks {
        render_block(block, out);
    }
}

fn render_block(block: &Block, out: &mut String) {
    match block {
        Block::Heading { level, inlines } => {
            out.push_str(&format!("<h{}>", level));
            render_inlines(inlines, out);
            out.push_str(&format!("</h{}>\n", level));
        }
        Block::Paragraph(inlines) => {
            out.push_str("<p>");
            render_inlines(inlines, out);
            out.push_str("</p>\n");
        }
        Block::HorizontalRule => {
            out.push_str("<hr>\n");
        }
        Block::PageBreak => {
            out.push_str("<hr class=\"page-break\" style=\"page-break-after: always;\">\n");
        }
        Block::CodeBlock { lang, code } => {
            if let Some(l) = lang {
                out.push_str(&format!("<pre><code class=\"language-{}\">", escape_attr(l)));
            } else {
                out.push_str("<pre><code>");
            }
            out.push_str(&escape_html(code));
            out.push_str("</code></pre>\n");
        }
        Block::QuoteSingle(inlines) => {
            out.push_str("<blockquote><p>");
            render_inlines(inlines, out);
            out.push_str("</p></blockquote>\n");
        }
        Block::QuoteMulti { alt, blocks } => {
            if let Some(cite) = alt {
                out.push_str(&format!("<blockquote data-alt=\"{}\">\n", escape_attr(cite)));
            } else {
                out.push_str("<blockquote>\n");
            }
            render_blocks(blocks, out);
            out.push_str("</blockquote>\n");
        }
        Block::Details { alt, blocks } => {
            out.push_str("<details>\n<summary>");
            render_inlines(alt, out);
            out.push_str("</summary>\n");
            render_blocks(blocks, out);
            out.push_str("</details>\n");
        }
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            out.push_str(&format!("<{}>\n", tag));
            for item in items {
                render_list_item(item, out);
            }
            out.push_str(&format!("</{}>\n", tag));
        }
        Block::Table {
            alignments,
            headers,
            rows,
        } => {
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (idx, cell) in headers.iter().enumerate() {
                let align = alignments.get(idx).copied().unwrap_or(Alignment::None);
                out.push_str(&format!("<th{}>", align_attr(align)));
                render_inlines(cell, out);
                out.push_str("</th>\n");
            }
            out.push_str("</tr>\n</thead>\n<tbody>\n");
            for row in rows {
                out.push_str("<tr>\n");
                for (idx, cell) in row.iter().enumerate() {
                    let align = alignments.get(idx).copied().unwrap_or(Alignment::None);
                    out.push_str(&format!("<td{}>", align_attr(align)));
                    render_inlines(cell, out);
                    out.push_str("</td>\n");
                }
                out.push_str("</tr>\n");
            }
            out.push_str("</tbody>\n</table>\n");
        }
    }
}

fn render_list_item(item: &ListItem, out: &mut String) {
    if let Some(checked) = item.task {
        out.push_str("<li class=\"task-list-item\">");
        if checked {
            out.push_str("<input type=\"checkbox\" checked disabled> ");
        } else {
            out.push_str("<input type=\"checkbox\" disabled> ");
        }
    } else {
        out.push_str("<li>");
    }

    if item.blocks.len() == 1 {
        if let Block::Paragraph(inlines) = &item.blocks[0] {
            render_inlines(inlines, out);
            out.push_str("</li>\n");
            return;
        }
    }

    out.push('\n');
    render_blocks(&item.blocks, out);
    out.push_str("</li>\n");
}

fn render_footnotes(footnotes: &[FootnoteDef], out: &mut String) {
    out.push_str("<section class=\"footnotes\">\n<ol>\n");
    for fn_def in footnotes {
        let id = escape_attr(&fn_def.label);
        out.push_str(&format!("<li id=\"fn-{}\">\n", id));
        render_blocks(&fn_def.blocks, out);
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n</section>\n");
}

fn render_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(t) => out.push_str(&escape_html(t)),
            Inline::LineBreak | Inline::HtmlBr => out.push_str("<br>\n"),
            Inline::Italic(inner) => {
                out.push_str("<em>");
                render_inlines(inner, out);
                out.push_str("</em>");
            }
            Inline::Bold(inner) => {
                out.push_str("<strong>");
                render_inlines(inner, out);
                out.push_str("</strong>");
            }
            Inline::BoldItalic(inner) => {
                out.push_str("<strong><em>");
                render_inlines(inner, out);
                out.push_str("</em></strong>");
            }
            Inline::Strikethrough(inner) => {
                out.push_str("<del>");
                render_inlines(inner, out);
                out.push_str("</del>");
            }
            Inline::Code(code) => {
                out.push_str("<code>");
                out.push_str(&escape_html(code));
                out.push_str("</code>");
            }
            Inline::Link { title, url } => {
                let href = if url.contains('@') && !url.contains("://") && !url.starts_with("mailto:") {
                    format!("mailto:{}", url)
                } else {
                    url.clone()
                };
                out.push_str(&format!("<a href=\"{}\">", escape_attr(&href)));
                if let Some(t) = title {
                    render_inlines(t, out);
                } else {
                    out.push_str(&escape_html(url));
                }
                out.push_str("</a>");
            }
            Inline::Image { alt, width, path } => {
                if let Some(w) = width {
                    out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\" width=\"{}\">",
                        escape_attr(path),
                        escape_attr(alt),
                        w
                    ));
                } else {
                    out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\">",
                        escape_attr(path),
                        escape_attr(alt)
                    ));
                }
            }
            Inline::FootnoteRef(label) => {
                let id = escape_attr(label);
                out.push_str(&format!(
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{}\" id=\"fnref-{}\">[{}]</a></sup>",
                    id,
                    id,
                    escape_html(label)
                ));
            }
        }
    }
}

fn align_attr(align: Alignment) -> &'static str {
    match align {
        Alignment::None => "",
        Alignment::Left => " style=\"text-align: left;\"",
        Alignment::Center => " style=\"text-align: center;\"",
        Alignment::Right => " style=\"text-align: right;\"",
    }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    escape_html(s)
}
