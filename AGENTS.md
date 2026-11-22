# Neport 開発の入口

調査を引き継ぐ場合は[技術引き継ぎ](docs/ENGINEERING_HANDOFF.md)から読む。

| 作業 | 最初に読む資料 |
|---|---|
| 要求・利用場面の確認 | [目的と原則](docs/PRODUCT.md) → [要求・実装状況・未決事項](docs/REQUIREMENTS.md) |
| 改修箇所・順序の特定 | [アーキテクチャと実装へのリンク](docs/ARCHITECTURE.md) → [段階別の着手・完了条件](docs/ROADMAP.md) |
| 盤面・計時の設計とテスト | [進捗に関する判断](docs/adr/0001-ticket-progress.md)、[盤面と個人計画](docs/specs/boards-and-focus.md)、[タイマー](docs/specs/work-timer.md) |
| 起動・運用 | [README](README.md)、[Cloud Run手順](docs/CLOUD_RUN_DEPLOY.md)、[手動確認](docs/MANUAL_TESTING.md) |

## 実装・レビュー

- 対象の要求・仕様・コード・既存テストから着手する。「仕様案」「推奨」「未決」は合意済み要件と区別する。要求の変更は要求台帳、設計判断はADR、動作と受け入れ条件は個別仕様、実装済みの構造はアーキテクチャを更新する。
- Rustは既存の命名（関数 snake_case、型 PascalCase、定数 UPPER_SNAKE_CASE）、型付きの入出力、serde、AppError と Result を使う。機能別モジュールでHTTP処理とrepoのSQLを整理し、ルーターを登録する。SQLxは実行時検証。スキーマ変更は `backend/migrations/` の時刻付きSQLで扱う。バックエンドのロックファイルは現行の [.gitignore](.gitignore) に従う。
- API変更は認可・監査・再試行への影響と、Webの型・MCP・OpenAPIを確認する。画面文言は日英を揃える。テストでは正常系に加え、変更が関係する拒否・競合・既存データのケースを確認する。
- フロントのビルドは `docker compose build frontend`（DockerfileのNode 24）。バックエンドの統合テストは `make test-up` → `make test` → `make test-down` で専用DBを使う。自己完結のカバレッジ計測は `make cov`。MCPは `make mcp-check`、公開API文書は `make site`。[Makefile](Makefile)
- 変更に応じた検証結果と未検証範囲を報告する。文書のみならパス・リンク、用語、事実と提案の整合を確認する。
