# バイナリ構造体定義とハイライト機能 仕様書

> **Status:** Draft v1.0
> **Date:** 2026-05-07

## 1. 概要

### 1.1 目的

バイナリファイルを構造体定義ファイルに基づいて自動的に解析し、各フィールドを色分けハイライトで視覚化する機能を `xvi` に追加する。これにより、ファイルフォーマットのリバースエンジニアリングや構造化バイナリデータの解析を直感的に行える。

### 1.2 ゴール

- **宣言的な構造体定義:** TOML 形式の定義ファイル (`.xsd.toml`) でバイナリフォーマットを記述
- **自動解析・ハイライト:** 定義に基づきバイナリデータを解析し、HexView 上でフィールドごとに色分け表示
- **構造体ツリー表示:** 解析結果を階層的なツリービューで表示し、フィールドの名前・型・値を一覧
- **インタラクティブ連携:** ツリーのフィールド選択で HexView のカーソルが移動し、逆に HexView のカーソル位置に対応するフィールドがツリーで強調される

### 1.3 スコープ

| 含む | 含まない |
|------|----------|
| 固定長フィールド解析 | 圧縮・暗号化データの自動展開 |
| 配列・繰り返し構造 | チューリング完全なスクリプト言語 |
| 条件分岐 (`if`) | ネットワークストリーム解析 |
| ネスト構造体 | 構造体定義のGUIエディタ |
| エンディアン指定 | 外部プラグインシステム |
| カラーハイライト | データの書き換え・パッチ適用 |

---

## 2. 構造体定義ファイル形式

### 2.1 ファイル拡張子とフォーマット

- 拡張子: `.xsd.toml`
- フォーマット: TOML（Rust エコシステムとの親和性が高く、`serde` + `toml` crate で直接デシリアライズ可能）

### 2.2 基本構造

```toml
[meta]
id = "png_header"
title = "PNG File Header"
endian = "big"               # "big" | "little" (デフォルト: "little")
file_extension = "png"       # オプション: 自動適用のヒント
magic = [0x89, 0x50, 0x4E, 0x47]  # オプション: マジックナンバー検証

[[fields]]
id = "signature"
type = "bytes"
size = 8
display = "hex"
color = "#FF6B6B"
description = "PNG Signature"

[[fields]]
id = "chunk_length"
type = "u32"
color = "#4ECDC4"

[[fields]]
id = "chunk_type"
type = "str"
size = 4
encoding = "ascii"
color = "#45B7D1"

[[fields]]
id = "chunk_data"
type = "bytes"
size_ref = "chunk_length"    # 他フィールドの値を参照
color = "#96CEB4"

[[fields]]
id = "crc"
type = "u32"
color = "#FFEAA7"
```

### 2.3 `[meta]` セクション

| キー | 型 | 必須 | 説明 |
|------|----|------|------|
| `id` | `string` | ✅ | 定義の一意識別子 |
| `title` | `string` | ❌ | 人間向けの表示名 |
| `endian` | `string` | ❌ | デフォルトエンディアン (`"big"` / `"little"`)。デフォルト: `"little"` |
| `file_extension` | `string` | ❌ | 対象ファイル拡張子（自動適用ヒント） |
| `magic` | `int[]` | ❌ | ファイル先頭のマジックナンバー |

### 2.4 プリミティブ型一覧

| 型名 | サイズ | 説明 |
|------|--------|------|
| `u8` | 1 | 符号なし8ビット整数 |
| `i8` | 1 | 符号付き8ビット整数 |
| `u16` | 2 | 符号なし16ビット整数 |
| `i16` | 2 | 符号付き16ビット整数 |
| `u32` | 4 | 符号なし32ビット整数 |
| `i32` | 4 | 符号付き32ビット整数 |
| `u64` | 8 | 符号なし64ビット整数 |
| `i64` | 8 | 符号付き64ビット整数 |
| `f32` | 4 | 32ビット浮動小数点数 |
| `f64` | 8 | 64ビット浮動小数点数 |
| `bool` | 1 | ブーリアン (0 = false, 非0 = true) |
| `bytes` | 可変 | 生バイト列 (`size` 必須) |
| `str` | 可変 | 文字列 (`size` 必須、`encoding` 指定可) |
| `padding` | 可変 | パディング (解析スキップ、`size` 必須) |

### 2.5 フィールド定義の属性

| 属性 | 型 | 必須 | 説明 |
|------|----|------|------|
| `id` | `string` | ✅ | フィールド識別子 (snake_case) |
| `type` | `string` | ✅ | データ型 (プリミティブ型名またはカスタム型名) |
| `size` | `int` | △ | バイトサイズ (`bytes`/`str`/`padding` 型で必須) |
| `size_ref` | `string` | ❌ | サイズを別フィールドの値から取得 |
| `color` | `string` | ❌ | ハイライト色 (HEX `"#RRGGBB"` 形式)。未指定時は自動割当 |
| `description` | `string` | ❌ | フィールドの説明文 |
| `display` | `string` | ❌ | 表示形式 (`"decimal"` / `"hex"` / `"binary"` / `"ascii"`)。デフォルト: `"decimal"` |
| `endian` | `string` | ❌ | フィールド個別のエンディアン指定（`meta` のデフォルトを上書き） |
| `enum` | `string` | ❌ | 参照する enum 定義の ID |
| `if` | `string` | ❌ | 条件式。`true` の場合のみフィールドが存在 |
| `repeat` | `string` | ❌ | 繰り返し方式 (`"count"` / `"until"` / `"eof"`) |
| `repeat_expr` | `string` | ❌ | 繰り返し回数の式 (`repeat = "count"` 時に使用) |
| `repeat_until` | `string` | ❌ | 繰り返し終了条件式 (`repeat = "until"` 時に使用) |

### 2.6 配列・繰り返し

```toml
# 固定回数の繰り返し
[[fields]]
id = "entries"
type = "entry"
repeat = "count"
repeat_expr = "num_entries"

# 条件付き繰り返し
[[fields]]
id = "chunks"
type = "chunk"
repeat = "until"
repeat_until = "_.chunk_type == 'IEND'"

# EOF まで繰り返し
[[fields]]
id = "records"
type = "record"
repeat = "eof"
```

### 2.7 条件分岐

```toml
[[fields]]
id = "extended_header"
type = "ext_header"
if = "version >= 2"

[[fields]]
id = "optional_data"
type = "bytes"
size = 16
if = "flags & 0x01 != 0"
```

### 2.8 カスタム型（ネスト構造体）

```toml
[types.chunk]
[[types.chunk.fields]]
id = "length"
type = "u32"

[[types.chunk.fields]]
id = "chunk_type"
type = "str"
size = 4
encoding = "ascii"

[[types.chunk.fields]]
id = "data"
type = "bytes"
size_ref = "length"

[[types.chunk.fields]]
id = "crc"
type = "u32"
```

### 2.9 Enum 定義

```toml
[enums.color_type]
0 = "Grayscale"
2 = "RGB"
3 = "Indexed"
4 = "Grayscale+Alpha"
6 = "RGBA"
```

### 2.10 式 (Expression) の文法

条件分岐 (`if`) やサイズ参照 (`size_ref`, `repeat_expr`) で使用する式の文法:

```
expr     = term (("+" | "-" | "==" | "!=" | ">=" | "<=" | ">" | "<" | "&&" | "||" | "&" | "|") term)*
term     = factor (("*" | "/" | "%") factor)*
factor   = NUMBER | HEX_NUMBER | STRING | FIELD_REF | "(" expr ")" | "!" factor
FIELD_REF = identifier ("." identifier)*
```

- フィールド参照: `field_name` で同階層のフィールド値を参照
- ドット記法: `parent.child` でネストフィールドへアクセス
- `_` は現在のアイテム（繰り返し内で使用）
- リテラル: 整数 (`42`), 16進数 (`0xFF`), 文字列 (`'IEND'`)

---

## 3. 解析エンジン

### 3.1 アーキテクチャ

```
                       ┌─────────────────────┐
                       │  Definition File     │
                       │  (.xsd.toml)         │
                       └──────────┬──────────┘
                                  │ parse (toml + serde)
                                  ▼
                       ┌─────────────────────┐
                       │  StructDefinition    │
                       │  (Rust struct)       │
                       └──────────┬──────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│  StructParser    │   │  FieldResolver   │   │  ExprEvaluator   │
│  (解析実行)       │   │  (型・サイズ解決) │   │  (式評価)         │
└────────┬─────────┘   └──────────────────┘   └──────────────────┘
         │
         ▼
┌──────────────────┐
│  ParseResult     │
│  (解析済みツリー)  │
└────────┬─────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌────────┐ ┌──────────────┐
│HexView │ │StructTreeView│
│Highlight│ │  (新規UI)     │
└────────┘ └──────────────┘
```

### 3.2 コアデータ型

```rust
/// 構造体定義ファイル全体
pub struct StructDefinition {
    pub meta: MetaDef,
    pub fields: Vec<FieldDef>,
    pub types: HashMap<String, TypeDef>,
    pub enums: HashMap<String, EnumDef>,
}

/// フィールド定義
pub struct FieldDef {
    pub id: String,
    pub field_type: FieldType,
    pub size: Option<usize>,
    pub size_ref: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub display: DisplayFormat,
    pub endian: Option<Endianness>,
    pub enum_ref: Option<String>,
    pub condition: Option<String>,      // if 式
    pub repeat: Option<RepeatMode>,
    pub repeat_expr: Option<String>,
    pub repeat_until: Option<String>,
}

/// 解析結果の1フィールド
pub struct ParsedField {
    pub id: String,
    pub offset: usize,          // バッファ内の開始オフセット
    pub size: usize,            // バイト数
    pub value: FieldValue,      // 解析済みの値
    pub color: Hsla,            // ハイライト色
    pub description: String,
    pub children: Vec<ParsedField>,  // ネスト構造体の場合
}

/// 解析結果全体
pub struct ParseResult {
    pub definition_id: String,
    pub fields: Vec<ParsedField>,
    pub total_parsed_bytes: usize,
    pub errors: Vec<ParseError>,     // 解析中のエラー・警告
}
```

### 3.3 解析フロー

1. **定義ファイル読み込み:** TOML をパースし `StructDefinition` に変換
2. **バリデーション:** 型参照・フィールド参照の整合性チェック
3. **順次解析:** `fields` を先頭から順にバッファに適用
   - プリミティブ型: エンディアンに従いバイト列から値を読み取り
   - `size_ref`: 参照先フィールドの解析済み値からサイズを決定
   - `if`: 式を評価し、`false` ならスキップ
   - `repeat`: 指定された方式で繰り返し解析
   - カスタム型: `types` から定義を取得し再帰的に解析
4. **結果構築:** `ParseResult` としてフィールドツリーを構築
5. **ハイライト生成:** `ParsedField` の `offset` + `size` からハイライト範囲を生成

### 3.4 エラーハンドリング

解析は **ベストエフォート** 方式とする。エラーが発生しても解析を中断せず、エラー情報を `ParseResult.errors` に記録し、解析可能な部分は結果に含める。

| エラー種別 | 動作 |
|-----------|------|
| バッファ不足 | 該当フィールド以降をスキップ、警告を記録 |
| 型不一致 | 生バイトとして表示、警告を記録 |
| 式評価エラー | 条件を `false` として扱い、警告を記録 |
| マジックナンバー不一致 | 警告を出して解析を続行 |

---

## 4. UI 仕様

### 4.1 HexView ハイライト

既存の `HexView.highlights: Vec<(Range<usize>, Hsla)>` を活用し、構造体フィールドごとにハイライトを追加する。

- 各フィールドに異なる色を割り当て（定義ファイルの `color` 属性、または自動パレット）
- ネスト構造体は親の色の濃淡バリエーションで表現
- 検索ハイライトとの共存: 構造体ハイライトは背景色、検索ハイライトは前景色で差別化
- ホバー時にフィールド名と値をツールチップ表示（将来拡張）

#### 自動カラーパレット

`color` 未指定時に使用するデフォルトパレット（12色、視認性と区別しやすさを重視）:

```rust
const DEFAULT_PALETTE: [&str; 12] = [
    "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4",
    "#FFEAA7", "#DDA0DD", "#98D8C8", "#F7DC6F",
    "#BB8FCE", "#85C1E9", "#F0B27A", "#AED6F1",
];
```

### 4.2 構造体ツリーパネル (`StructTreePanel`)

新規パネルとして `StructTreePanel` を実装し、Dock システムに統合する。

```
┌─ Structure: PNG File Header ──────────────────────┐
│ ▼ [■] signature        bytes[8]   89 50 4E 47 ... │
│   [■] chunk_length     u32        13              │
│   [■] chunk_type       str[4]     "IHDR"          │
│ ▼ [■] chunk_data       bytes[13]                  │
│   │ [■] width          u32        1920            │
│   │ [■] height         u32        1080            │
│   │ [■] bit_depth      u8         8               │
│   │ [■] color_type     u8         6 (RGBA)        │
│   │ ...                                           │
│   [■] crc              u32        0x1A2B3C4D      │
└───────────────────────────────────────────────────┘
```

**表示要素:**
- `[■]` — フィールドのハイライト色インジケーター
- `▼` / `▶` — ネスト構造体の展開/折りたたみ
- フィールドID、型、解析済み値
- Enum の場合は数値と名前を併記

**インタラクション:**
- フィールドクリック → HexView のカーソルが該当オフセットに移動、範囲を選択
- HexView のカーソル移動 → ツリー内の対応フィールドがハイライト
- 右クリックコンテキストメニュー: 値のコピー、16進表示/10進表示の切り替え

### 4.3 定義ファイル操作 UI

**ツールバーまたはメニュー:**
- `Structure > Load Definition...` — 定義ファイルを選択して適用
- `Structure > Reload Definition` — 定義ファイルを再読み込み
- `Structure > Clear Definition` — 構造体ハイライトを解除
- `Structure > Auto-detect` — マジックナンバーから定義を自動選択

**キーバインド:**
| キー | アクション |
|------|-----------|
| `Cmd+Shift+S` | Load Structure Definition |
| `Cmd+Shift+R` | Reload Structure Definition |

---

## 5. モジュール構成

### 5.1 新規ファイル

```
src/core/
├── structure/
│   ├── mod.rs              # モジュール公開
│   ├── definition.rs       # StructDefinition, FieldDef 等のデータ型
│   ├── parser.rs           # StructParser (解析エンジン)
│   ├── expression.rs       # ExprEvaluator (式評価)
│   └── palette.rs          # カラーパレット管理

src/ui/
├── panels/
│   └── struct_tree_panel.rs  # StructTreePanel (構造体ツリー)
├── components/
│   └── struct_tree_view.rs   # StructTreeView (ツリー描画コンポーネント)
```

### 5.2 既存ファイルの変更

| ファイル | 変更内容 |
|---------|---------|
| `src/core/mod.rs` | `pub mod structure;` 追加 |
| `src/core/editor.rs` | `parse_result: Option<ParseResult>` フィールド追加 |
| `src/ui/panels/editor_panel.rs` | 構造体ハイライトの HexView への反映ロジック追加 |
| `src/ui/workspace.rs` | `StructTreePanel` の Dock 登録、メニュー項目追加 |
| `src/actions.rs` | 構造体関連アクション追加 |
| `Cargo.toml` | `toml` crate 依存追加 |

---

## 6. 具体的な使用例

### 6.1 BMP ファイルヘッダー

```toml
[meta]
id = "bmp_header"
title = "BMP File Header"
endian = "little"
file_extension = "bmp"
magic = [0x42, 0x4D]

[[fields]]
id = "signature"
type = "str"
size = 2
encoding = "ascii"
color = "#FF6B6B"
description = "BMP Signature ('BM')"

[[fields]]
id = "file_size"
type = "u32"
color = "#4ECDC4"
description = "Total file size in bytes"

[[fields]]
id = "reserved"
type = "padding"
size = 4

[[fields]]
id = "data_offset"
type = "u32"
color = "#45B7D1"
description = "Offset to pixel data"
display = "hex"

[[fields]]
id = "info_header_size"
type = "u32"
color = "#96CEB4"

[[fields]]
id = "width"
type = "i32"
color = "#FFEAA7"
description = "Image width in pixels"

[[fields]]
id = "height"
type = "i32"
color = "#DDA0DD"
description = "Image height in pixels"

[[fields]]
id = "planes"
type = "u16"

[[fields]]
id = "bits_per_pixel"
type = "u16"
color = "#F7DC6F"

[enums.compression]
0 = "BI_RGB (None)"
1 = "BI_RLE8"
2 = "BI_RLE4"
3 = "BI_BITFIELDS"

[[fields]]
id = "compression"
type = "u32"
enum = "compression"
color = "#BB8FCE"
```

### 6.2 カスタムプロトコルの例

```toml
[meta]
id = "custom_protocol"
title = "Custom Binary Protocol"
endian = "big"

[[fields]]
id = "version"
type = "u8"

[[fields]]
id = "flags"
type = "u8"
display = "binary"

[[fields]]
id = "payload_length"
type = "u16"

[[fields]]
id = "payload"
type = "bytes"
size_ref = "payload_length"

# v2 以上にのみ存在する拡張ヘッダー
[[fields]]
id = "extended_header"
type = "ext_header"
if = "version >= 2"

[[fields]]
id = "checksum"
type = "u32"
display = "hex"

[types.ext_header]
[[types.ext_header.fields]]
id = "ext_type"
type = "u16"

[[types.ext_header.fields]]
id = "ext_length"
type = "u16"

[[types.ext_header.fields]]
id = "ext_data"
type = "bytes"
size_ref = "ext_length"
```

---

## 7. 実装フェーズ

### Phase 1: コア解析エンジン（MVP）

- [ ] `StructDefinition` のデータモデルと TOML デシリアライズ
- [ ] プリミティブ型の解析 (`u8`〜`u64`, `i8`〜`i64`, `f32`, `f64`, `bytes`, `str`)
- [ ] エンディアン対応
- [ ] `ParseResult` の構築
- [ ] ユニットテスト

### Phase 2: HexView ハイライト統合

- [ ] `ParseResult` → `Vec<(Range<usize>, Hsla)>` 変換
- [ ] `EditorPanel` に定義ファイル読み込み機能
- [ ] 自動カラーパレット
- [ ] 定義ファイル読み込み/クリアのアクションとキーバインド

### Phase 3: 構造体ツリーパネル

- [ ] `StructTreePanel` の実装
- [ ] フィールド選択 ↔ HexView カーソル連携
- [ ] Dock システムへの統合

### Phase 4: 高度な機能

- [ ] `size_ref` によるフィールド間参照
- [ ] `if` 条件分岐
- [ ] `repeat` 繰り返し
- [ ] カスタム型（ネスト構造体）
- [ ] Enum 定義と値のラベル表示
- [ ] 式評価エンジン (`ExprEvaluator`)

### Phase 5: UX 改善

- [ ] マジックナンバーによる定義ファイル自動検出
- [ ] 定義ファイルのホットリロード（ファイル監視）
- [ ] フィールドホバーツールチップ
- [ ] 定義ファイルの構文エラー表示
- [ ] ビルトイン定義テンプレート (BMP, PNG, ELF, PE 等)

---

## 8. 既存システムとの比較

| 特徴 | xvi (本仕様) | 010 Editor | ImHex | Kaitai Struct |
|------|-------------|-----------|-------|---------------|
| 定義形式 | TOML (宣言的) | C-like (.bt) | C-like (Pattern Language) | YAML (.ksy) |
| 学習コスト | 低 | 中〜高 | 中〜高 | 低〜中 |
| チューリング完全 | ❌ | ✅ | ✅ | ❌ |
| エディタ統合 | ネイティブ | ネイティブ | ネイティブ | 外部ツール |
| クロスプラットフォーム | ✅ | ✅ | ✅ | ✅ |

**設計判断:** TOML を選択した理由:
1. Rust エコシステムとの親和性（`serde` + `toml` で直接デシリアライズ）
2. 宣言的で学習コストが低い
3. `Cargo.toml` や各種 Rust ツールで馴染みのある形式
4. チューリング完全なスクリプトは将来の拡張として検討可能

---

## 9. 将来の拡張候補

- **Kaitai Struct `.ksy` インポート:** 既存の豊富な定義資産を活用
- **構造体定義の GUI エディタ:** HexView 上で範囲選択 → フィールド定義を対話的に作成
- **ビットフィールド対応:** ビット単位のフィールド定義 (`u4`, `u12` 等)
- **仮想アドレスマッピング:** ELF/PE のセクションアドレス変換
- **テンプレートリポジトリ:** コミュニティ共有の定義ファイル集
