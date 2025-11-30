# statworks 設計ドキュメント

statworks は，GitHub をはじめ複数サービスの統計情報を一元的に扱い，
SVG のカードとして返す軽量 API サーバである．
Rust 製の Cloudflare Workers 上で動作し，高速かつ柔軟に拡張可能な構成を採る．

---

## 1．概要

statworks は以下を目標とした Web API 群である．

* GitHub Readme Stats に相当する機能を Rust で再実装
* Cloudflare Workers に最適化した高速レスポンス
* さまざまなサービスの統計情報を統合する基盤
* SVG デザインを API で返す仕組みを提供
* テーマ，フォント，表示形式をクエリで指定可能

現在は GitHub の統計を中心に対応するが，将来的には複数サービス統合を目指す．

---

## 2．現状提供する機能（GitHub Stats）

* 言語割合（ミニ円グラフ）
* 総スター数
* コミット数（年間または合計）
* Summary カード（総合カード）
* テーマ切り替え（light，dark，tokyo-night，solarized-light，nord，monokai）
* フォント指定（Times New Roman など）
* Cloudflare Workers のキャッシュを用いた高速化

---

## 3．API 設計

### 3.1 共通仕様

すべての API は SVG を返す．
Content-Type は `image/svg+xml`．
テーマや表示内容はクエリで指定して変更できる．

```
GET /api/<endpoint>?user=USERNAME&theme=THEME&その他パラメータ
```

---

### 3.2 エンドポイント一覧

#### 言語割合

```
GET /api/lang?user=USERNAME&theme=THEME
```

返す内容：ミニ円グラフ（開始角 0,1 方向＝真上スタート）と凡例．

#### スター数

```
GET /api/stars?user=USERNAME&theme=THEME
```

#### コミット数

```
GET /api/commits?user=USERNAME&mode=yearly&theme=THEME
```

mode は以下．

* yearly（12 か月の contrib）
* total（contributors API を合算）

#### Summary カード

```
GET /api/summary?user=USERNAME&theme=THEME
```

言語，スター，コミットを 1 枚のカードにまとめる．

---

## 4．テーマ設計

テーマは Query パラメータによって切り替えできる．

### 4.1 対応テーマ一覧

* light（デフォルト）
* dark
* tokyo-night
* solarized-light
* nord
* monokai

未知のテーマ名が指定された場合は light にフォールバックする．

### 4.2 Theme struct

```rust
pub struct Theme {
    pub background: &'static str,
    pub border: &'static str,
    pub text: &'static str,
    pub accent1: &'static str,  // Rust 等の主要色
    pub accent2: &'static str,
    pub accent3: &'static str,
}
```

---

## 5．フォント設計

SVG 内では外部フォントの import が GitHub でブロックされる可能性があるため，
以下の system font を使用する方式を採る．

```
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
```

また，Times New Roman など任意 serif を指定する場合は以下のように SVG 内に置く．

```svg
<style>
  text { font-family: "Times New Roman", Times, serif; }
</style>
```

フォント指定用の構造体を作成して，設定から変更できるようにする．

---

## 6．SVG デザイン

### 6.1 全体方針

* ミニマルデザイン
* カード枠とフォントの統一
* 軽量（rect，text，circle，path のみ）
* Rust 側で文字列として組み立て可能
* Cloudflare Workers でも高速に生成できる

### 6.2 言語割合（mini pie）

特徴：

* 開始角は真上（0,1）
* 円グラフはパス 3 枚で構成
* Legend は circle + text
* フォントは Times New Roman または system sans-serif

### 6.3 スター数，コミット数カード

* 左上にラベル
* 下段に数値を大きめに表示
* 背景色と枠線は Theme に合わせる
* レスポンスは即時生成（キャッシュ利用）

### 6.4 Summary カード（compact）

* タイトル
* Stars，Commits，Languages を縦方向に配置
* テーマで配色を調整

---

## 7．Rust 実装設計

### 7.1 構成

```
statworks/
 ├─ src/
 │   ├─ theme.rs        // テーマ配色
 │   ├─ render/
 │   │    ├─ lang.rs    // 言語割合 SVG 生成
 │   │    ├─ stars.rs   // スター SVG 生成
 │   │    ├─ commits.rs // コミット SVG 生成
 │   │    └─ summary.rs // Summary カード生成
 │   ├─ github.rs       // GitHub API クライアント
 │   └─ lib.rs          // ルーティング
 ├─ Cargo.toml
 └─ wrangler.toml
```

### 7.2 テーマ切り替え

テーマ指定用の構造体を作成し，設定から変更できるようにする．

```rust
pub fn theme_from_query(name: &str) -> Theme {
    match name.to_lowercase().as_str() {
        "dark" => Theme { ... },
        "tokyo-night" => Theme { ... },
        "solarized-light" => Theme { ... },
        "nord" => Theme { ... },
        "monokai" => Theme { ... },
        _ => Theme::light(), // fallback
    }
}
```

### 7.3 Cloudflare Workers

Rust モジュールは wasm32-unknown-unknown にビルドし Workers に deploy する．

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wrangler publish
```

---

## 8．GitHub API（現在対応している部分）

* リポジトリ一覧取得
* 言語 API
* コントリビューション（GraphQL yearly）
* スター数の集計

レートリミット回避のため Workers Cache API と KV を併用する．

---

## 9．将来的な拡張（マルチサービス対応）

statworks は以下のサービス統合を視野に入れている．

* GitHub
* GitLab
* Bitbucket
* AtCoder（レーティング・提出）
* Stack Overflow
* Zenn / Qiita
* Twitter / Bluesky（フォロワー統計）

共通化の方向性：

* 統計データを抽象化した Trait
* 各サービスのクライアント実装
* Summary カードへの統合表示
* 共通キャッシュ戦略

---

## 10．ライセンス

MIT ライセンス．

