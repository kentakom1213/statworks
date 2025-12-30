# statworks 設計ドキュメント（Summary 専用・簡約版）

statworks は，GitHub をはじめとするサービスの統計情報を集計し，
SVG の Summary カードとして返却する軽量 API サーバである．
Rust 製の Cloudflare Workers 上で動作し，高速かつシンプルな構成を採る．

本ドキュメントは，**Summary カードのみを提供する最小構成**としての
statworks の設計仕様を記述する．

---

## 1．概要

statworks は以下を目的とした Web API である．

- GitHub Readme Stats 相当の Summary カードを Rust で再実装
- Cloudflare Workers 上での低レイテンシ SVG 生成
- SVG を API レスポンスとして直接返却
- テーマ指定を最小限に抑えたシンプルな設計
- Askama による SVG テンプレート生成

本バージョンでは，**Summary カードのみを実装対象**とし，
個別の言語カードや数値カードは提供しない．

---

## 2．提供機能（Summary Card）

Summary カードには以下の情報を含める．

- GitHub ユーザー名
- 総スター数
- コミット数（年間）
- Pull Requests 数
- Issues 数
- 言語割合（mini pie chart，上起点）

---

## 3．API 設計

### 3.1 エンドポイント

```
GET /summary?user=USERNAME
```

#### クエリパラメータ

| 名前             | 必須 | 説明                |
| ---------------- | ---- | ------------------- |
| user             | yes  | GitHub ユーザー名   |
| background-color | no   | 背景色（CSS color） |
| text-color       | no   | 文字色（CSS color） |

指定がない場合は，以下をデフォルトとする．

```text
background-color = #F6F1D1
text-color       = #0B2027
```

#### レスポンス

- Content-Type: `image/svg+xml`
- Body: Summary カード SVG

---

## 4．テーマ設計（簡約版）

テーマは **2 色のみ**で構成する．
アクセント色（言語色など）は statworks 側で固定定義する．

### 4.1 Theme struct

```rust
pub struct Theme {
    pub background_color: String,
    pub text_color: String,
}
```

- `background_color`：カード背景
- `text_color`：テキスト・ベース色

クエリで指定された値をそのまま使用し，
未指定時はデフォルト値を用いる．

---

## 5．フォント設計

SVG 内では system font を使用する．

```css
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial,
  sans-serif;
```

フォントは固定とし，クエリによる切り替えは行わない．
（将来拡張の余地は残す）

---

## 6．SVG デザイン設計（Summary）

### 6.1 共通方針

- `<rect>`, `<text>`, `<circle>` のみで構成
- テキストはすべて `<text>`（アウトライン化しない）
- 外枠ストロークは使用しない
- Askama テンプレートは描画専用
- 数値計算・割合計算は Rust 側で実施

---

### 6.2 言語割合（mini pie）

- 開始角は真上（12 時方向）
- 円周長 (C = 2\pi r)
- 割合 (p \in [0,1])

[
\text{dasharray} = (C \cdot p,; C)
]

[
\text{dashoffset} = -C \cdot \sum_{j<i} p_j
]

SVG 側では以下を適用する．

```svg
<g transform="rotate(0 cx cy)">
```

- `stroke-linecap="butt"`
- セグメント数は可変

---

## 7．Rust 実装設計

### 7.1 ディレクトリ構成（Summary 専用）

```
statworks/
 ├─ src/
 │   ├─ theme.rs        // background / text color
 │   ├─ github.rs       // GitHub API client
 │   ├─ render/
 │   │    └─ summary.rs // Summary 用データ生成
 │   ├─ template/
 │   │    └─ summary.rs // Askama Template struct
 │   └─ lib.rs          // /summary routing
 ├─ templates/
 │   └─ svg/
 │        └─ summary.svg
 ├─ Cargo.toml
 └─ wrangler.toml
```

---

### 7.2 Summary 生成フロー

1．`/summary?user=USERNAME` を受信
2．GitHub API から必要な情報を取得
3．Rust 側で数値集計・言語割合計算
4．Askama テンプレートに値を流し込む
5．SVG をレスポンスとして返却

---

## 8．GitHub API 対応範囲

- Repositories 一覧
- Languages API
- GraphQL Contributions（年間）
- Pull Requests / Issues 集計
- Star 合算

Workers Cache API を用いてレスポンスをキャッシュする．

---

## 9．将来的拡張（設計余地）

- テーマプリセット復活（dark 等）
- フォント指定クエリ
- 言語凡例の ON/OFF
- 他サービス統合（GitLab 等）

現段階では **Summary に集中**し，
API と SVG 生成の安定性を優先する．

---

## 10．ライセンス

MIT License．
