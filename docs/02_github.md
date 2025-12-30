# statworks：GitHub API 設計（Public only／No PAT／REST API）

## 1．前提条件

- 対象は **Public repository のみ**
- **Personal Access Token（PAT）は使用しない**
- GitHub API は **未認証アクセス**を前提とする
- レートリミット回避のため **強いキャッシュ依存設計**とする
- README 埋め込み用途のため，可用性と失敗時のフォールバックを重視する

---

## 2．使用する GitHub API（REST）

### 2.1 採用 API

- GitHub REST API（v3）を使用する

```
GET https://api.github.com/users/{username}/repos
GET https://api.github.com/repos/{owner}/{repo}/languages
GET https://api.github.com/users/{username}/events/public
```

理由：

- 未認証でも利用可能であり，GraphQL よりもエラー判別が容易
- エンドポイントが単純でデバッグしやすい
- Public データのみを安全に取得可能

---

## 3．取得対象データ（Public repository）

### 3.1 repositories 一覧

```
GET /users/{username}/repos?per_page=100&page=N&sort=updated
```

取得項目：

- `full_name`
- `fork`
- `archived`
- `stargazers_count`
- `owner.login`
- `name`

除外条件：

- `fork == true`
- `archived == true`

---

### 3.2 languages

```
GET /repos/{owner}/{repo}/languages
```

返却例：

```json
{
  "Rust": 123456,
  "TypeScript": 45678
}
```

- 値は bytes
- `0` は無視可能

---

### 3.3 commits（yearly）

認証なしで正確な commit 合計は困難なため，以下の代替手段を採用する：

```
GET /users/{username}/events/public?per_page=100&page=N
```

- 直近イベントから `PushEvent` の `payload.size` を合算
- 最大 300 件（3 ページ）程度で打ち切り
- **近似値**として扱う

※ どうしても正確な年次 commit が必要なら認証が必須であることを明示する

---

## 4．ページング戦略

- `per_page=100`
- `page=1..N`
- 未認証のため **最大取得 repo 数に上限を設ける**

例：

- 最大 300〜500 repo で打ち切り
- 上限到達時は概算集計として処理継続

---

## 5．レートリミット対策（未認証）

### 5.1 制約

- REST API の未認証リクエスト上限は 60 requests/hour (IP 単位)
- README が多数表示されると即座に枯渇する可能性がある

### 5.2 方針

- **GitHub API はキャッシュミス時のみ叩く**
- キャッシュヒット時は GitHub API に一切アクセスしない
- 連続アクセス時は stale を返す（stale-while-revalidate）

---

## 6．キャッシュ設計（必須）

### 6.1 キャッシュ層

- Cloudflare Workers Cache API
- 永続 KV（Cloudflare KV）

### 6.2 キャッシュキー

集計結果に影響するすべての要素を含める：

- username
- endpoint（lang，summary など）
- theme
- 表示パラメータ（上位言語数など）

### 6.3 TTL 方針

- Workers Cache：短期（数分〜1 時間）
- KV：長期（6〜24 時間）

SVG 応答に付与：

```
Cache-Control: public, s-maxage=86400, stale-while-revalidate=3600
```

---

## 7．取得データの後処理（API 観点）

### 7.1 repository フィルタ

- `fork == true` は除外
- `archived == true` は除外
- 明示的に exclude 指定された repo は除外

### 7.2 languages 集計

- `/languages` を合算
- size が 0 の言語は無視
- 最終的に割合へ変換

---

## 8．失敗時の挙動

- GitHub API 失敗時：

  - キャッシュがあればキャッシュを返す
  - キャッシュが無ければ簡易エラー SVG を返す

- README 表示を壊さないことを最優先

---

## 9．この設計の割り切り

- private repository は一切反映されない
- fork を多用するユーザでは言語比率が実態より小さく出る可能性がある
- commit 数は未認証のため **概算**である
- 正確性より **安定性・軽量性を優先**
