/// RFM のコメント（`// ...` および `/* ... */`）を除去する。
/// コードブロック（```）、インラインコード（`）、`<>` 内、および `\` エスケープ直後は対象外。
pub fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    let mut in_code_block = false;
    let mut code_fence_len = 0;
    let mut in_inline_code = 0usize; // 0 = false, >0 = backtick count
    let mut in_angle = false;
    let mut in_block_comment = false;

    while i < len {
        // ブロックコメント中
        if in_block_comment {
            if i + 1 < len && chars[i] == '*' && chars[i + 1] == '/' {
                in_block_comment = false;
                i += 2;
                // コメントだけの行だった場合、直後の改行1つを詰める
                if is_line_empty_so_far(&out) && i < len && chars[i] == '\n' {
                    i += 1;
                }
            } else {
                i += 1;
            }
            continue;
        }

        // コードブロック中
        if in_code_block {
            if is_at_line_start(&chars, i) {
                let mut k = i;
                while k < len && chars[k] == ' ' {
                    k += 1;
                }
                let mut ticks = 0;
                while k + ticks < len && chars[k + ticks] == '`' {
                    ticks += 1;
                }
                if ticks >= code_fence_len && is_rest_of_line_blank(&chars, k + ticks) {
                    in_code_block = false;
                    code_fence_len = 0;
                }
            }
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // インラインコード中
        if in_inline_code > 0 {
            if chars[i] == '`' {
                let mut ticks = 0;
                while i + ticks < len && chars[i + ticks] == '`' {
                    ticks += 1;
                }
                for _ in 0..ticks {
                    out.push('`');
                }
                if ticks == in_inline_code {
                    in_inline_code = 0;
                }
                i += ticks;
                continue;
            }
            if chars[i] == '\n' {
                // 改行をまたぐ場合でも閉じまでは継続（RFMは段落内改行あり）
            }
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // `<>` 内部
        if in_angle {
            if chars[i] == '>' || chars[i] == '\n' {
                in_angle = false;
            }
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // 行頭でのコードブロック開始判定（スペース4の倍数 + ```）
        if is_at_line_start(&chars, i) {
            let mut spaces = 0;
            while i + spaces < len && chars[i + spaces] == ' ' {
                spaces += 1;
            }
            if spaces % 4 == 0 {
                let mut ticks = 0;
                while i + spaces + ticks < len && chars[i + spaces + ticks] == '`' {
                    ticks += 1;
                }
                if ticks >= 3 {
                    in_code_block = true;
                    code_fence_len = ticks;
                    for j in i..(i + spaces + ticks) {
                        out.push(chars[j]);
                    }
                    i += spaces + ticks;
                    continue;
                }
            }
        }

        // バックスラッシュエスケープ
        if chars[i] == '\\' && i + 1 < len {
            out.push(chars[i]);
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }

        // インラインコード開始
        if chars[i] == '`' {
            let mut ticks = 0;
            while i + ticks < len && chars[i + ticks] == '`' {
                ticks += 1;
            }
            if has_closing_backticks(&chars, i + ticks, ticks) {
                in_inline_code = ticks;
            }
            for _ in 0..ticks {
                out.push('`');
            }
            i += ticks;
            continue;
        }

        // `<>` 開始（同じ行内に閉じ `>` がある場合のみ保護）
        if chars[i] == '<' && has_closing_angle_on_line(&chars, i + 1) {
            in_angle = true;
            out.push('<');
            i += 1;
            continue;
        }

        // 複数行コメント開始 `/*`
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            in_block_comment = true;
            i += 2;
            continue;
        }

        // 単行コメント `//`（行頭、または直前が半角スペース）
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let at_start = is_at_line_start(&chars, i);
            let after_space = i > 0 && chars[i - 1] == ' ';
            if at_start || after_space {
                // 行末までスキップ
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                if at_start {
                    // 行頭コメントは改行ごと消す
                    if i < len && chars[i] == '\n' {
                        i += 1;
                    }
                } else {
                    // 行末コメント前の末尾半角スペースを削る
                    while out.ends_with(' ') {
                        out.pop();
                    }
                }
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

fn is_at_line_start(chars: &[char], i: usize) -> bool {
    i == 0 || chars[i - 1] == '\n'
}

fn is_rest_of_line_blank(chars: &[char], mut i: usize) -> bool {
    while i < chars.len() && chars[i] != '\n' {
        if chars[i] != ' ' && chars[i] != '\t' {
            return false;
        }
        i += 1;
    }
    true
}

fn is_line_empty_so_far(out: &str) -> bool {
    match out.rfind('\n') {
        Some(pos) => out[pos + 1..].chars().all(|c| c == ' '),
        None => out.chars().all(|c| c == ' '),
    }
}

fn has_closing_backticks(chars: &[char], mut i: usize, target: usize) -> bool {
    let len = chars.len();
    while i < len {
        if chars[i] == '`' {
            let mut ticks = 0;
            while i + ticks < len && chars[i + ticks] == '`' {
                ticks += 1;
            }
            if ticks == target {
                return true;
            }
            i += ticks;
        } else {
            i += 1;
        }
    }
    false
}

fn has_closing_angle_on_line(chars: &[char], mut i: usize) -> bool {
    while i < chars.len() {
        if chars[i] == '\n' {
            return false;
        }
        if chars[i] == '>' {
            return true;
        }
        i += 1;
    }
    false
}
