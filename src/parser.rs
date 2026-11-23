use crate::ast::{Alignment, Block, Document, FootnoteDef, Inline, ListItem};
use crate::comment::strip_comments;

pub fn parse(input: &str) -> Document {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let (frontmatter, body) = extract_frontmatter(&normalized);
    let cleaned = strip_comments(body);
    let lines: Vec<&str> = cleaned.lines().collect();

    let mut footnotes = Vec::new();
    let blocks = parse_blocks(&lines, &mut footnotes);

    Document {
        frontmatter,
        blocks,
        footnotes,
    }
}

fn extract_frontmatter(input: &str) -> (Option<String>, &str) {
    if !input.starts_with("---\n") {
        return (None, input);
    }
    let rest = &input[4..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end();
        if trimmed == "---" {
            let fm = rest[..offset].trim_end_matches('\n').to_string();
            let after = &rest[offset + line.len()..];
            return (Some(fm), after);
        }
        offset += line.len();
    }
    (None, input)
}

fn parse_blocks(lines: &[&str], footnotes: &mut Vec<FootnoteDef>) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;
    let len = lines.len();

    while i < len {
        let line = lines[i];

        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // 1. コードブロック (``` で始まり ``` で終わる)
        if let Some((lang, fence_len)) = parse_code_fence_open(line) {
            let mut code_lines = Vec::new();
            i += 1;
            while i < len {
                if is_code_fence_close(lines[i], fence_len) {
                    i += 1;
                    break;
                }
                code_lines.push(lines[i]);
                i += 1;
            }
            blocks.push(Block::CodeBlock {
                lang,
                code: code_lines.join("\n"),
            });
            continue;
        }

        // 2. 複数行引用 (>>> alt で始まり >>> で終わる、中身はインデント4つ)
        if let Some(alt) = parse_multi_quote_open(line) {
            let mut inner_lines = Vec::new();
            i += 1;
            while i < len {
                if lines[i].trim() == ">>>" {
                    i += 1;
                    break;
                }
                inner_lines.push(strip_four_spaces(lines[i]));
                i += 1;
            }
            let inner_blocks = parse_blocks(&inner_lines, footnotes);
            blocks.push(Block::QuoteMulti {
                alt,
                blocks: inner_blocks,
            });
            continue;
        }

        // 3. 折りたたみ ([alt]{ ... })
        if let Some((alt_str, single_body)) = parse_details_open(line) {
            let alt_inlines = parse_inlines(alt_str);
            if let Some(body) = single_body {
                let inner_blocks = if body.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![Block::Paragraph(parse_inlines(body.trim()))]
                };
                blocks.push(Block::Details {
                    alt: alt_inlines,
                    blocks: inner_blocks,
                });
                i += 1;
            } else {
                let mut inner_lines = Vec::new();
                i += 1;
                while i < len {
                    if lines[i] == "}" || lines[i].trim() == "}" && !lines[i].starts_with("    ") {
                        i += 1;
                        break;
                    }
                    inner_lines.push(strip_four_spaces(lines[i]));
                    i += 1;
                }
                let inner_blocks = parse_blocks(&inner_lines, footnotes);
                blocks.push(Block::Details {
                    alt: alt_inlines,
                    blocks: inner_blocks,
                });
            }
            continue;
        }

        // 4. 水平線 (--- のみ)
        if line.trim() == "---" {
            blocks.push(Block::HorizontalRule);
            i += 1;
            continue;
        }

        // 5. 改ページ (===)
        if line.trim() == "===" {
            blocks.push(Block::PageBreak);
            i += 1;
            continue;
        }

        // 6. 見出し (# のみ、スペース必須)
        if let Some((level, content)) = parse_heading(line) {
            blocks.push(Block::Heading {
                level,
                inlines: parse_inlines(content),
            });
            i += 1;
            continue;
        }

        // 7. 1行引用 (> )
        if let Some(content) = parse_single_quote(line) {
            blocks.push(Block::QuoteSingle(parse_inlines(content)));
            i += 1;
            continue;
        }

        // 8. 脚注定義 ([^label]: ...)
        if let Some((label, first_body)) = parse_footnote_def_start(line) {
            let mut fn_lines = Vec::new();
            if !first_body.is_empty() {
                fn_lines.push(first_body);
            }
            i += 1;
            while i < len && (lines[i].starts_with("    ") || lines[i].trim().is_empty()) {
                if lines[i].trim().is_empty() {
                    if i + 1 < len && lines[i + 1].starts_with("    ") {
                        fn_lines.push("");
                        i += 1;
                        continue;
                    } else {
                        break;
                    }
                }
                fn_lines.push(strip_four_spaces(lines[i]));
                i += 1;
            }
            let fn_blocks = parse_blocks(&fn_lines, footnotes);
            footnotes.push(FootnoteDef {
                label: label.to_string(),
                blocks: fn_blocks,
            });
            continue;
        }

        // 9. テーブル
        if i + 1 < len && is_table_header_pair(lines[i], lines[i + 1]) {
            let headers_raw = split_table_row(lines[i]);
            let alignments = parse_table_alignments(lines[i + 1]);
            let headers = headers_raw.into_iter().map(|c| parse_inlines(c.trim())).collect();
            i += 2;
            let mut rows = Vec::new();
            while i < len && looks_like_table_row(lines[i]) {
                let row_cells = split_table_row(lines[i])
                    .into_iter()
                    .map(|c| parse_inlines(c.trim()))
                    .collect();
                rows.push(row_cells);
                i += 1;
            }
            blocks.push(Block::Table {
                alignments,
                headers,
                rows,
            });
            continue;
        }

        // 10. リスト (- または 1. )
        if let Some((ordered, _)) = parse_list_marker(line) {
            let (list_block, next_i) = parse_list(lines, i, ordered, footnotes);
            blocks.push(list_block);
            i = next_i;
            continue;
        }

        // 11. 段落 (空行または別ブロック開始まで)
        let mut para_lines = Vec::new();
        while i < len && !lines[i].trim().is_empty() && !starts_block(lines, i) {
            para_lines.push(lines[i]);
            i += 1;
        }
        let joined = para_lines.join("\n");
        blocks.push(Block::Paragraph(parse_inlines(&joined)));
    }

    blocks
}

fn starts_block(lines: &[&str], i: usize) -> bool {
    let line = lines[i];
    parse_code_fence_open(line).is_some()
        || parse_multi_quote_open(line).is_some()
        || parse_details_open(line).is_some()
        || line.trim() == "---"
        || line.trim() == "==="
        || parse_heading(line).is_some()
        || parse_single_quote(line).is_some()
        || parse_footnote_def_start(line).is_some()
        || (i + 1 < lines.len() && is_table_header_pair(line, lines[i + 1]))
        || parse_list_marker(line).is_some()
}

fn strip_four_spaces(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("    ") {
        rest
    } else {
        line
    }
}

fn parse_code_fence_open(line: &str) -> Option<(Option<String>, usize)> {
    let trimmed = line.trim_end();
    if !trimmed.starts_with("```") {
        return None;
    }
    let fence_len = trimmed.chars().take_while(|&c| c == '`').count();
    if fence_len < 3 {
        return None;
    }
    let rest = trimmed[fence_len..].trim();
    if rest.contains('`') {
        return None;
    }
    let lang = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };
    Some((lang, fence_len))
}

fn is_code_fence_close(line: &str, min_len: usize) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with("```") {
        return false;
    }
    let fence_len = trimmed.chars().take_while(|&c| c == '`').count();
    fence_len >= min_len && trimmed.len() == fence_len
}

fn parse_multi_quote_open(line: &str) -> Option<Option<String>> {
    let rest = line.strip_prefix(">>>")?;
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        Some(None)
    } else {
        Some(Some(trimmed.to_string()))
    }
}

fn parse_details_open(line: &str) -> Option<(&str, Option<&str>)> {
    if !line.starts_with('[') {
        return None;
    }
    let close_bracket = line.find("]{")?;
    let alt = &line[1..close_bracket];
    let after = &line[close_bracket + 2..];
    if after.trim().is_empty() {
        Some((alt, None))
    } else if let Some(inner) = after.strip_suffix('}') {
        Some((alt, Some(inner)))
    } else {
        None
    }
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((hashes as u8, rest.trim_end()))
}

fn parse_single_quote(line: &str) -> Option<&str> {
    if line.starts_with(">>>") {
        return None;
    }
    if line == ">" {
        return Some("");
    }
    line.strip_prefix("> ")
}

fn parse_footnote_def_start(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with("[^") {
        return None;
    }
    let end = line.find("]:")?;
    let label = &line[2..end];
    if label.is_empty() || label.contains(char::is_whitespace) {
        return None;
    }
    let body = line[end + 2..].trim_start();
    Some((label, body))
}

fn is_table_header_pair(header: &str, sep: &str) -> bool {
    if !header.contains('|') || !sep.contains('|') {
        return false;
    }
    let sep_cells = split_table_row(sep);
    if sep_cells.is_empty() {
        return false;
    }
    sep_cells.iter().all(|c| {
        let t = c.trim();
        if t.is_empty() {
            return false;
        }
        let inner = t.strip_prefix(':').unwrap_or(t);
        let inner = inner.strip_suffix(':').unwrap_or(inner);
        !inner.is_empty() && inner.chars().all(|ch| ch == '-')
    })
}

fn looks_like_table_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn split_table_row(line: &str) -> Vec<&str> {
    let mut s = line.trim();
    if let Some(stripped) = s.strip_prefix('|') {
        s = stripped;
    }
    if let Some(stripped) = s.strip_suffix('|') {
        s = stripped;
    }
    s.split('|').collect()
}

fn parse_table_alignments(sep: &str) -> Vec<Alignment> {
    split_table_row(sep)
        .into_iter()
        .map(|c| {
            let t = c.trim();
            match (t.starts_with(':'), t.ends_with(':')) {
                (true, true) => Alignment::Center,
                (true, false) => Alignment::Left,
                (false, true) => Alignment::Right,
                (false, false) => Alignment::None,
            }
        })
        .collect()
}

fn parse_list_marker(line: &str) -> Option<(bool, &str)> {
    if let Some(rest) = line.strip_prefix("- ") {
        return Some((false, rest));
    }
    let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 {
        if let Some(rest) = line[digits..].strip_prefix(". ") {
            return Some((true, rest));
        }
    }
    None
}

fn parse_list(
    lines: &[&str],
    start: usize,
    ordered: bool,
    footnotes: &mut Vec<FootnoteDef>,
) -> (Block, usize) {
    let mut items = Vec::new();
    let mut i = start;
    let len = lines.len();

    while i < len {
        let Some((item_ordered, rest)) = parse_list_marker(lines[i]) else {
            break;
        };
        if item_ordered != ordered {
            break;
        }

        let (task, content) = if !ordered {
            if let Some(r) = rest.strip_prefix("[ ] ") {
                (Some(false), r)
            } else if let Some(r) = rest.strip_prefix("[x] ").or_else(|| rest.strip_prefix("[X] "))
            {
                (Some(true), r)
            } else {
                (None, rest)
            }
        } else {
            (None, rest)
        };

        let mut item_lines = vec![content];
        i += 1;

        while i < len {
            if lines[i].starts_with("    ") {
                item_lines.push(strip_four_spaces(lines[i]));
                i += 1;
            } else if lines[i].trim().is_empty()
                && i + 1 < len
                && lines[i + 1].starts_with("    ")
            {
                item_lines.push("");
                i += 1;
            } else {
                break;
            }
        }

        let item_blocks = parse_blocks(&item_lines, footnotes);
        items.push(ListItem {
            task,
            blocks: item_blocks,
        });
    }

    (Block::List { ordered, items }, i)
}

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut text_buf = String::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let ch = input[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();

        // 改行
        if ch == '\n' {
            flush_text(&mut text_buf, &mut out);
            out.push(Inline::LineBreak);
            i += 1;
            continue;
        }

        // バックスラッシュエスケープ
        if ch == '\\' && i + 1 < len {
            let next_ch = input[i + 1..].chars().next().unwrap();
            if next_ch != '\n' {
                text_buf.push(next_ch);
                i += 1 + next_ch.len_utf8();
                continue;
            }
        }

        // インラインコード (`code` / `` `code` ``)
        if ch == '`' {
            let tick_count = input[i..].bytes().take_while(|&b| b == b'`').count();
            if let Some(close_rel) = find_closing_ticks(&input[i + tick_count..], tick_count) {
                flush_text(&mut text_buf, &mut out);
                let raw = &input[i + tick_count..i + tick_count + close_rel];
                let code = if raw.len() >= 2 && raw.starts_with(' ') && raw.ends_with(' ') && raw.chars().any(|c| c != ' ') {
                    raw[1..raw.len() - 1].to_string()
                } else {
                    raw.to_string()
                };
                out.push(Inline::Code(code));
                i += tick_count + close_rel + tick_count;
                continue;
            }
        }

        // <br> タグのみ許可
        if ch == '<' {
            if let Some(br_len) = match_br_tag(&input[i..]) {
                flush_text(&mut text_buf, &mut out);
                out.push(Inline::HtmlBr);
                i += br_len;
                continue;
            }
        }

        // 画像 (![alt]<path> or ![alt|300]<path>)
        if ch == '!' && i + 1 < len && bytes[i + 1] == b'[' {
            if let Some((alt, width, path, consumed)) = parse_image(&input[i..]) {
                flush_text(&mut text_buf, &mut out);
                out.push(Inline::Image { alt, width, path });
                i += consumed;
                continue;
            }
        }

        // 脚注参照 ([^label])
        if ch == '[' && i + 1 < len && bytes[i + 1] == b'^' {
            if let Some(close) = input[i + 2..].find(']') {
                let label = &input[i + 2..i + 2 + close];
                if !label.is_empty() && !label.contains(char::is_whitespace) && !label.contains('\n') {
                    flush_text(&mut text_buf, &mut out);
                    out.push(Inline::FootnoteRef(label.to_string()));
                    i += 2 + close + 1;
                    continue;
                }
            }
        }

        // タイトル付きリンク ([title]<url>)
        if ch == '[' {
            if let Some((title_raw, url, consumed)) = parse_titled_link(&input[i..]) {
                flush_text(&mut text_buf, &mut out);
                out.push(Inline::Link {
                    title: Some(parse_inlines(title_raw)),
                    url: url.to_string(),
                });
                i += consumed;
                continue;
            }
        }

        // 単体リンク・パス・メール (<url>)
        if ch == '<' {
            if let Some(close) = input[i + 1..].find('>') {
                let inner = &input[i + 1..i + 1 + close];
                if is_valid_angle_target(inner) {
                    flush_text(&mut text_buf, &mut out);
                    out.push(Inline::Link {
                        title: None,
                        url: inner.to_string(),
                    });
                    i += 1 + close + 1;
                    continue;
                }
            }
        }

        // 取り消し線 (~~...~~)
        if input[i..].starts_with("~~") {
            if let Some(close) = find_closing_delim(&input[i + 2..], "~~") {
                flush_text(&mut text_buf, &mut out);
                let inner = &input[i + 2..i + 2 + close];
                out.push(Inline::Strikethrough(parse_inlines(inner)));
                i += 2 + close + 2;
                continue;
            }
        }

        // 強調 (*** / ** / *)
        if ch == '*' {
            if input[i..].starts_with("***") {
                if let Some(close) = find_closing_delim(&input[i + 3..], "***") {
                    flush_text(&mut text_buf, &mut out);
                    let inner = &input[i + 3..i + 3 + close];
                    out.push(Inline::BoldItalic(parse_inlines(inner)));
                    i += 3 + close + 3;
                    continue;
                }
            }
            if input[i..].starts_with("**") {
                if let Some(close) = find_closing_delim(&input[i + 2..], "**") {
                    flush_text(&mut text_buf, &mut out);
                    let inner = &input[i + 2..i + 2 + close];
                    out.push(Inline::Bold(parse_inlines(inner)));
                    i += 2 + close + 2;
                    continue;
                }
            }
            if let Some(close) = find_closing_single_star(&input[i + 1..]) {
                flush_text(&mut text_buf, &mut out);
                let inner = &input[i + 1..i + 1 + close];
                out.push(Inline::Italic(parse_inlines(inner)));
                i += 1 + close + 1;
                continue;
            }
        }

        text_buf.push(ch);
        i += ch_len;
    }

    flush_text(&mut text_buf, &mut out);
    out
}

fn flush_text(buf: &mut String, out: &mut Vec<Inline>) {
    if !buf.is_empty() {
        out.push(Inline::Text(std::mem::take(buf)));
    }
}

fn find_closing_ticks(s: &str, target: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let count = s[i..].bytes().take_while(|&b| b == b'`').count();
            if count == target {
                return Some(i);
            }
            i += count;
        } else {
            i += 1;
        }
    }
    None
}

fn match_br_tag(s: &str) -> Option<usize> {
    let lower_prefix: String = s.chars().take(6).collect::<String>().to_ascii_lowercase();
    if lower_prefix.starts_with("<br>") {
        Some(4)
    } else if lower_prefix.starts_with("<br/>") {
        Some(5)
    } else if lower_prefix.starts_with("<br />") {
        Some(6)
    } else {
        None
    }
}

fn parse_image(s: &str) -> Option<(String, Option<u32>, String, usize)> {
    // ![alt]<path> or ![alt|300]<path>
    let close_bracket = s[2..].find("]<")?;
    let alt_part = &s[2..2 + close_bracket];
    if alt_part.contains('\n') {
        return None;
    }
    let path_start = 2 + close_bracket + 2;
    let close_angle = s[path_start..].find('>')?;
    let path = &s[path_start..path_start + close_angle];
    if path.is_empty() || path.contains('\n') {
        return None;
    }

    let (alt, width) = if let Some(pipe_pos) = alt_part.rfind('|') {
        let w_str = alt_part[pipe_pos + 1..].trim();
        if let Ok(w) = w_str.parse::<u32>() {
            (alt_part[..pipe_pos].to_string(), Some(w))
        } else {
            (alt_part.to_string(), None)
        }
    } else {
        (alt_part.to_string(), None)
    };

    Some((alt, width, path.to_string(), path_start + close_angle + 1))
}

fn parse_titled_link(s: &str) -> Option<(&str, &str, usize)> {
    let close_bracket = s[1..].find("]<")?;
    let title = &s[1..1 + close_bracket];
    if title.contains('\n') {
        return None;
    }
    let url_start = 1 + close_bracket + 2;
    let close_angle = s[url_start..].find('>')?;
    let url = &s[url_start..url_start + close_angle];
    if url.is_empty() || url.contains('\n') {
        return None;
    }
    Some((title, url, url_start + close_angle + 1))
}

fn is_valid_angle_target(s: &str) -> bool {
    if s.is_empty() || s.contains('\n') || s.contains('<') || s.contains(' ') {
        return false;
    }
    // URLスキーム (http://, https://, mailto: 等) または メール (@) または パス (./, ../, ドライブ文字, /a/b や /file.ext)
    s.contains("://")
        || s.starts_with("mailto:")
        || (s.contains('@') && s.contains('.'))
        || s.starts_with("./")
        || s.starts_with("../")
        || (s.starts_with('/') && (s[1..].contains('/') || s[1..].contains('.')))
        || (s.len() >= 3 && s.as_bytes()[1] == b':' && (s.as_bytes()[2] == b'/' || s.as_bytes()[2] == b'\\'))
}

fn find_closing_delim(s: &str, delim: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            let count = s[i..].bytes().take_while(|&b| b == b'`').count();
            if let Some(close) = find_closing_ticks(&s[i + count..], count) {
                i += count + close + count;
                continue;
            }
        }
        if s[i..].starts_with(delim) && i > 0 {
            return Some(i);
        }
        let ch = s[i..].chars().next().unwrap();
        i += ch.len_utf8();
    }
    None
}

fn find_closing_single_star(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            let count = s[i..].bytes().take_while(|&b| b == b'`').count();
            if let Some(close) = find_closing_ticks(&s[i + count..], count) {
                i += count + close + count;
                continue;
            }
        }
        if bytes[i] == b'*' {
            if s[i..].starts_with("**") {
                // ** のペアはスキップして探すか、ここで抜ける
                i += 2;
                continue;
            }
            if i > 0 {
                return Some(i);
            }
        }
        let ch = s[i..].chars().next().unwrap();
        i += ch.len_utf8();
    }
    None
}
