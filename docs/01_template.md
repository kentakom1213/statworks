# GitHub Stats Card SVG（Askama テンプレ設計）

このドキュメントは，Rust + Askama を用いて GitHub Stats Card 用 SVG を生成するための実装仕様である．
SVG は system font を用いた `<text>` ベース構成とし，Top Languages は円グラフで表現する．

---

## 設計方針

- SVG は Askama テンプレートで生成する
- 数値計算（円弧長など）は Rust 側で前処理する
- テンプレートは「描画専用」に徹する
- テーマ・統計値・言語数はすべて可変
- 円グラフの起点は上（12 時方向）

---

## 数学的定義（円グラフ）

半径を (r)，割合を (p \in [0,1]) とする．

[
C = 2\pi r
]

[
\text{dasharray} = (C \cdot p,; C)
]

[
\text{dashoffset} = -C \cdot \sum_{j<i} p_j
]

SVG 仕様上の開始点（3 時方向）を補正するため，
円グラフ全体に以下の変換を適用する．

```svg
transform="rotate(0 cx cy)"
```

---

## Askama SVG テンプレート

ファイル：`templates/svg/stats_card.svg`

```svg
<svg xmlns="http://www.w3.org/2000/svg"
     width="{{ width }}"
     height="{{ height }}"
     viewBox="0 0 {{ width }} {{ height }}"
     role="img"
     aria-label="{{ aria_label|e }}">

  <rect x="0" y="0"
        width="{{ width }}"
        height="{{ height }}"
        rx="{{ radius }}"
        fill="{{ theme.background_color }}" />

  <text x="{{ pad_x }}" y="{{ title_y }}"
        fill="{{ theme.primary_color }}"
        font-family="{{ font_family|e }}"
        font-size="{{ title_size }}"
        dominant-baseline="middle">
    {{ title|e }}
  </text>

  <g transform="translate({{ left_x }}, {{ left_y }})">
    {% for row in stats_rows %}
      <g transform="translate(0, {{ row.dy }})">
        <text x="0" y="0"
              fill="{{ theme.primary_color }}"
              font-family="{{ font_family|e }}"
              font-size="{{ stat_label_size }}"
              dominant-baseline="middle"
              opacity="{{ stat_label_opacity }}">
          {{ row.label|e }}
        </text>

        <text x="{{ stat_value_x }}" y="0"
              fill="{{ theme.primary_color }}"
              font-family="{{ font_family|e }}"
              font-size="{{ stat_value_size }}"
              dominant-baseline="middle">
          {{ row.value|e }}
        </text>
      </g>
    {% endfor %}
  </g>

  <g transform="translate({{ pie_group_x }}, {{ pie_group_y }})">
    <text x="0" y="{{ pie_title_y }}"
          fill="{{ theme.primary_color }}"
          font-family="{{ font_family|e }}"
          font-size="{{ section_title_size }}"
          dominant-baseline="middle">
      {{ top_languages_title|e }}
    </text>

    <g transform="translate({{ pie_cx }}, {{ pie_cy }})">
      {% if pie_base_stroke != "" %}
      <circle cx="0" cy="0" r="{{ pie_r }}"
              fill="none"
              stroke="{{ pie_base_stroke }}"
              stroke-width="{{ pie_stroke }}" />
      {% endif %}

      <g transform="rotate(-90 0 0)">
        {% for seg in lang_segments %}
        <circle cx="0" cy="0" r="{{ pie_r }}"
                fill="none"
                stroke="{{ seg.color }}"
                stroke-width="{{ pie_stroke }}"
                stroke-dasharray="{{ seg.dasharray }}"
                stroke-dashoffset="{{ seg.dashoffset }}" />
        {% endfor %}
      </g>
    </g>
  </g>
</svg>
```

---

## Rust 側データ構造

```rust
use askama::Template;

#[derive(Debug, Clone)]
pub struct Theme {
    pub background_color: String,
    pub primary_color: String,
}

#[derive(Debug, Clone)]
pub struct StatRow {
    pub label: String,
    pub value: String,
    pub dy: i32,
}

#[derive(Debug, Clone)]
pub struct LangSegment {
    pub name: String,
    pub color: String,
    pub percent_text: String,
    pub dasharray: String,
    pub dashoffset: String,
    pub legend_dy: i32,
}

#[derive(Template)]
#[template(path = "svg/stats_card.svg")]
pub struct StatsCardTemplate {
    pub theme: Theme,

    pub width: i32,
    pub height: i32,
    pub radius: i32,
    pub pad_x: i32,

    pub title: String,
    pub aria_label: String,
    pub font_family: String,
    pub title_y: i32,
    pub title_size: i32,

    pub stats_rows: Vec<StatRow>,
    pub left_x: i32,
    pub left_y: i32,
    pub stat_value_x: i32,
    pub stat_label_size: i32,
    pub stat_value_size: i32,
    pub stat_label_opacity: f32,

    pub top_languages_title: String,
    pub section_title_size: i32,
    pub pie_group_x: i32,
    pub pie_group_y: i32,
    pub pie_title_y: i32,

    pub pie_cx: i32,
    pub pie_cy: i32,
    pub pie_r: f64,
    pub pie_stroke: f64,
    pub pie_base_stroke: String,

    pub lang_segments: Vec<LangSegment>,
}
```

---

## 言語割合 → 円グラフ用セグメント生成関数

```rust
pub fn build_lang_segments(
    langs: &[(String, String, f64)], // (name, color, ratio)
    r: f64,
    legend_line_height: i32,
) -> Vec<LangSegment> {
    let c = 2.0 * std::f64::consts::PI * r;

    let mut acc = 0.0;
    let mut out = Vec::new();

    for (i, (name, color, p)) in langs.iter().enumerate() {
        let p = p.clamp(0.0, 1.0);

        let dasharray = format!("{:.4} {:.4}", c * p, c);
        let dashoffset = format!("{:.4}", -(c * acc));

        out.push(LangSegment {
            name: name.clone(),
            color: color.clone(),
            percent_text: format!("{:.1}%", p * 100.0),
            dasharray,
            dashoffset,
            legend_dy: i as i32 * legend_line_height,
        });

        acc += p;
    }

    out
}
```

---

## 可変パラメータ対応表

- テーマ配色

  - `theme.background_color`
  - `theme.primary_color`

- 統計情報

  - `stats_rows`（Stars, Commits, PRs, Issues, Contributes）

- 言語情報

  - `lang_segments`（数・割合ともに可変）

---

## codex への指示例（そのまま貼れる）

> 上記仕様に従い，Askama を用いた SVG 生成コードを Rust で実装してください．
> 計算は Rust 側で行い，テンプレートは描画専用としてください．
> SVG は GitHub README 上で正しく表示される必要があります．
