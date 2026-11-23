# RFM (Rabbit Flavored Markdown)

GFM（GitHub Flavored Markdown）をベースに曖昧さを排除し、「1表現1記法」へ統一した独自Markdown方言 **Rabbit Flavored Markdown (RFM)** の Rust 製パーサーおよび HTML レンダラー。

## 構成

- `src/ast.rs`: AST（構文木）定義（`Document`, `Block`, `Inline` 等）
- `src/comment.rs`: コメント除去（`//`, `/* ... */`、コード・`<>`・エスケープ保護）
- `src/parser.rs`: ブロックおよびインラインパーサー
- `src/html.rs`: HTML レンダラー
- `src/main.rs`: CLI エントリポイント（`--ast` で AST 出力、デフォルトで HTML 出力）
- `tests/rfm_spec.rs`: RFM 仕様テスト

## 使い方

```bash
# HTML へ変換
cargo run -- sample.rfm

# AST を確認
cargo run -- --ast sample.rfm

# テスト実行
cargo test
```
