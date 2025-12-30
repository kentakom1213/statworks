# 実装方針（statworks）

本書は docs/00_initial.md、docs/01_template.md、docs/02_github.md をもとに、statworks の実装方針を整理したものである。
Rust + Cloudflare Workers + Askama を前提に、GitHub 公開情報のみを対象とする。

---

## 1. 目的と前提

- GitHub Readme Stats 相当の SVG カード API を Rust で実装する
- Cloudflare Workers 上で動作する軽量 API とする
- GitHub API は未認証（Public のみ、PAT 不使用）
- レート制限を前提にキャッシュ最優先設計
- README 埋め込み用途のため、失敗時も表示を壊さない

---

## 2. 全体アーキテクチャ

- エンドポイント
  - `/api/summary` 総合カード
- 返却形式：SVG（`Content-Type: image/svg+xml`）
- テーマ、フォント、表示形式はクエリで指定
- キャッシュ層：Workers Cache + Cloudflare KV

---

## 3. Rust 実装構成

```
statworks/
 ├─ src/
 │   ├─ theme.rs        // テーマ配色
 │   ├─ render/
 │   │    ├─ lang.rs    // 言語割合 SVG
 │   │    ├─ stars.rs   // スター SVG
 │   │    ├─ commits.rs // コミット SVG
 │   │    └─ summary.rs // Summary SVG
 │   ├─ github.rs       // GitHub GraphQL クライアント
 │   └─ lib.rs          // ルーティング、共通処理
 ├─ templates/
 │   └─ svg/
 │       └─ stats_card.svg
 ├─ Cargo.toml
 └─ wrangler.toml
```

- SVG 生成は Askama テンプレートを採用
- 計算は Rust 側で行い、テンプレートは描画のみ
- 文字列組み立てよりもテンプレートを優先

---

## 4. SVG 生成方針

- system font を基本とする
- フォントはクエリ指定に応じて埋め込み可能
- Askama テンプレートは `templates/svg/stats_card.svg` を基準とし、以下を可変化
  - テーマ配色
  - stats 行数
  - 言語セグメント数
  - サイズ、パディング、座標

### 4.1 円グラフ（Top Languages）

- 起点は上（12 時方向）
- `rotate(0 cx cy)` を適用
- セグメント計算は Rust 側で行う

```rust
// dasharray = (C * p, C), dashoffset = -C * sum(prev)
```

---

## 5. GitHub API 方針

- GitHub GraphQL API を未認証で使用
- Authorization ヘッダは付与しない
- 必須ヘッダ：
  - `User-Agent: statworks`
  - `Content-Type: application/json`

### 5.1 取得対象

- Public repository のみ
- `nameWithOwner`, `isFork`, `isArchived`, `languages` のみ取得
- private repo は無視

### 5.2 ページング

- `repositories(first: 100)`
- `hasNextPage` の間 `after = endCursor`
- 未認証のため最大 repo 数を制限（例 300〜500）
- 上限超えは概算集計として継続

---

## 6. キャッシュ戦略

- GitHub API は **キャッシュミス時のみ実行**
- キャッシュキーには集計結果に影響する全ての要素を含む
  - username
  - endpoint
  - theme
  - 表示パラメータ（上位言語数など）
- TTL 方針
  - Workers Cache：数分〜1 時間
  - KV：6〜24 時間
- SVG 応答ヘッダ
  - `Cache-Control: public, s-maxage=86400, stale-while-revalidate=3600`

---

## 7. 失敗時の挙動

- GitHub API 失敗時
  - キャッシュがあれば返す
  - キャッシュが無ければ簡易エラー SVG を返す
- README 表示を壊さないことを最優先

---

## 8. テーマ・フォント

- 未知のテーマ指定は `light` にフォールバック
- `Theme` struct に配色を定義
- フォントは system font をデフォルトとし、クエリ指定で上書き

---

## 9. 将来的な拡張を見据えた設計

- GitHub 以外（GitLab, AtCoder 等）への拡張を想定
- 統計データを Trait で抽象化し、Summary カードで統合可能にする
- キャッシュと SVG 生成は共通化

---

## 10. 実装優先順位

1. GitHub 言語割合（GraphQL + SVG 円グラフ）
2. stars / commits の基本カード
3. summary カード
4. テーマ・フォント切替
5. キャッシュ最適化
6. エラーカード・フォールバック

---

## 11. ビルド・デプロイ

- wasm32-unknown-unknown ターゲットでビルド
- `wrangler publish` で Workers にデプロイ

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wrangler publish
```
