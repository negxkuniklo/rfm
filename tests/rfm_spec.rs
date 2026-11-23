use rfm::{parse, to_html, Alignment, Block, Inline};

#[test]
fn test_frontmatter_and_basic_blocks() {
    let input = "\
---
title: Test
---
# 見出し1
#スペースなしは段落

1行目
2行目

---

===
";
    let doc = parse(input);
    assert_eq!(doc.frontmatter.as_deref(), Some("title: Test"));
    assert_eq!(
        doc.blocks[0],
        Block::Heading {
            level: 1,
            inlines: vec![Inline::Text("見出し1".into())]
        }
    );
    assert_eq!(
        doc.blocks[1],
        Block::Paragraph(vec![Inline::Text("#スペースなしは段落".into())])
    );
    assert_eq!(
        doc.blocks[2],
        Block::Paragraph(vec![
            Inline::Text("1行目".into()),
            Inline::LineBreak,
            Inline::Text("2行目".into()),
        ])
    );
    assert_eq!(doc.blocks[3], Block::HorizontalRule);
    assert_eq!(doc.blocks[4], Block::PageBreak);
}

#[test]
fn test_comments_and_protections() {
    let input = "\
// 行頭コメントは消える
本文 // 行末コメントも消える
/*
複数行
コメント
*/
本文2 /* インラインコメント */ 続き
<https://example.com/a//b>
`// コード内は無効`

```rust
// コードブロック内も無効
/* ここも無効 */
```
";
    let html = to_html(input);
    assert!(!html.contains("行頭コメント"));
    assert!(!html.contains("行末コメント"));
    assert!(!html.contains("複数行"));
    assert!(!html.contains("インラインコメント"));
    assert!(html.contains("本文<br>\n本文2  続き"));
    assert!(html.contains("<a href=\"https://example.com/a//b\">https://example.com/a//b</a>"));
    assert!(html.contains("<code>// コード内は無効</code>"));
    assert!(html.contains("// コードブロック内も無効"));
    assert!(html.contains("/* ここも無効 */"));
}

#[test]
fn test_emphasis_escape_and_inline_code() {
    let input = "\
*斜体* **太字** ***太字斜体*** _強調しない_ ~~取り消し~~
\\*エスケープ\\* `` `バッククォート` ``
";
    let html = to_html(input);
    assert!(html.contains("<em>斜体</em>"));
    assert!(html.contains("<strong>太字</strong>"));
    assert!(html.contains("<strong><em>太字斜体</em></strong>"));
    assert!(html.contains("_強調しない_"));
    assert!(html.contains("<del>取り消し</del>"));
    assert!(html.contains("*エスケープ*"));
    assert!(html.contains("<code>`バッククォート`</code>"));
}

#[test]
fn test_links_images_and_no_autolink() {
    let input = "\
素のURL https://example.com はリンクにならない。
<https://example.com> と <user@example.com> と </path/to/file>
[タイトル]<https://example.com>
![代替]<img.jxl>
![幅指定|300]<img.jxl>
不等号 1 < 2 かつ 3 > 1 はリンクにならない。
<div>生HTMLは無効</div><br>改行
";
    let html = to_html(input);
    assert!(html.contains("素のURL https://example.com はリンクにならない。"));
    assert!(html.contains("<a href=\"https://example.com\">https://example.com</a>"));
    assert!(html.contains("<a href=\"mailto:user@example.com\">user@example.com</a>"));
    assert!(html.contains("<a href=\"/path/to/file\">/path/to/file</a>"));
    assert!(html.contains("<a href=\"https://example.com\">タイトル</a>"));
    assert!(html.contains("<img src=\"img.jxl\" alt=\"代替\">"));
    assert!(html.contains("<img src=\"img.jxl\" alt=\"幅指定\" width=\"300\">"));
    assert!(html.contains("1 &lt; 2 かつ 3 &gt; 1"));
    assert!(html.contains("&lt;div&gt;生HTMLは無効&lt;/div&gt;<br>\n改行"));
}

#[test]
fn test_lists_and_tasks() {
    let input = "\
- 項目1
    - ネスト1
- [ ] 未完了
- [x] 完了小文字
- [X] 完了大文字

1. 番号1
2. 番号2

* アスタリスクはリストにしない
";
    let html = to_html(input);
    assert!(html.contains("<ul>"));
    assert!(html.contains("<ol>"));
    assert!(html.contains("<input type=\"checkbox\" disabled> 未完了"));
    assert!(html.contains("<input type=\"checkbox\" checked disabled> 完了小文字"));
    assert!(html.contains("<input type=\"checkbox\" checked disabled> 完了大文字"));
    assert!(html.contains("<p>* アスタリスクはリストにしない</p>"));
}

#[test]
fn test_table_and_footnotes() {
    let input = "\
| 左 | 中央 | 右 |
| :--- | :---: | ---: |
| A | B<br>C | 100 |

本文[^1]

[^1]: 注釈の内容
";
    let doc = parse(input);
    if let Block::Table { alignments, .. } = &doc.blocks[0] {
        assert_eq!(alignments, &vec![Alignment::Left, Alignment::Center, Alignment::Right]);
    } else {
        panic!("expected table");
    }
    let html = to_html(input);
    assert!(html.contains("style=\"text-align: left;\""));
    assert!(html.contains("style=\"text-align: center;\""));
    assert!(html.contains("style=\"text-align: right;\""));
    assert!(html.contains("B<br>\nC"));
    assert!(html.contains("href=\"#fn-1\""));
    assert!(html.contains("<li id=\"fn-1\">"));
}

#[test]
fn test_quotes_details_and_no_indented_codeblock() {
    let input = "\
    スペース4つだけではコードブロックにならない

> 1行引用

>>> 出典テスト
    複数行の引用
    2行目
>>>

[詳細を開く]{
    折りたたみの本文
    ```rust
    fn main() {}
    ```
}
";
    let doc = parse(input);
    assert!(matches!(doc.blocks[0], Block::Paragraph(_)));
    assert!(matches!(doc.blocks[1], Block::QuoteSingle(_)));
    assert!(matches!(
        doc.blocks[2],
        Block::QuoteMulti {
            alt: Some(_),
            ..
        }
    ));
    if let Block::Details { blocks, .. } = &doc.blocks[3] {
        assert_eq!(
            blocks[1],
            Block::CodeBlock {
                lang: Some("rust".into()),
                code: "fn main() {}".into(),
            }
        );
    } else {
        panic!("expected details");
    }

    let html = to_html(input);
    assert!(html.contains("<details>\n<summary>詳細を開く</summary>"));
    assert!(html.contains("<pre><code class=\"language-rust\">fn main() {}</code></pre>"));
}
