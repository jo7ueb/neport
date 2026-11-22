# アーキテクチャ

## 現行システム

| 境界 | 責務と入口 |
|---|---|
| ブラウザ | React Router の画面を共通レイアウトに載せ、TanStack Query がサーバー状態を取得・更新する。HTTPクライアントとTypeScriptの型はフロント側にある。[ルート](../frontend/src/App.tsx)・[レイアウト](../frontend/src/components/Layout.tsx)・[APIクライアント](../frontend/src/api/client.ts)・[型](../frontend/src/types/models.ts) |
| HTTP API | nginx が同一オリジンのリクエストを単一の Rust/axum サーバーへ中継。機能別ルーターを `/api/v1` にまとめ、extractor が認証、handler が操作の調整、repo が主にSQLを担当する。遷移検証等はrepoにもある。[nginx](../nginx.conf)・[ルート登録](../backend/src/main.rs)・[チケットのルーター](../backend/src/tickets/mod.rs) |
| 永続化・運用 | SQLx による実行時検証のSQLを PostgreSQL 16 に発行する。Compose の migrator がSQLを適用する。添付は共通trait経由でローカルまたはGCSへ保存。管理画面のバックアップはDBと添付を含み、復元はメンテナンスモードで行う。[Compose](../docker-compose.yml)・[マイグレーション](../backend/migrations/)・[ストレージ](../backend/src/storage/mod.rs)・[復元](../backend/src/backup/handlers.rs) |
| 外部利用者 | 公開API仕様は utoipa 定義から生成する。Python製stdio MCPは同じHTTP APIを利用するため、パス・認可・レスポンスの変更が波及する。[OpenAPI](../backend/src/api_docs.rs)・[MCP](../mcp/neport_mcp/server.py)・[生成手順](../Makefile) |

## モデルと読み書き

- **案件と工程。** `projects` は1つの workflow を参照し、`project_members` が利用者と案件内ロールを結ぶ。`tickets` は1案件・単一の担当者（未割当可）・具体状態・任意の親・マイルストーンを持つ。先行／後続は `ticket_dependencies` の有向辺。WBSは親子を再帰取得し、削除は子孫も論理削除する。ガントは案件の票と依存辺を読み、残作業の見積は未完了の葉のポイントと案件の時間換算係数から計算する。[初期DDL](../backend/migrations/20250703000001_init.sql)・[チケット](../backend/src/tickets/repo.rs)・[ガント](../backend/src/gantt/repo.rs)・[案件集計](../backend/src/projects/repo.rs)・[マイルストーン集計](../backend/src/milestones/repo.rs)
- **盤面とレビュー。** workflow は状態・初期状態・許可遷移を定義する。現在のカテゴリは `todo / in_progress / done / confirmed`。標準案の Backlog に対応する独立カテゴリはない。案件別盤面は具体状態を列にし、カード移動でチケットの `status_id` を更新。未完了の子孫は親の完了遷移を妨げ、確認済みへの遷移は管理権限を要する。完了時に案件管理者へ通知する。削除は Write で即時実行される。[状態分類](../backend/src/workflows/models.rs)・[盤面](../frontend/src/pages/KanbanPage.tsx)・[更新・通知・削除](../backend/src/tickets/handlers.rs)・[遷移検証](../backend/src/tickets/repo.rs)
- **横断表示。** 個人ダッシュボードは各案件から本人担当・進行中の全ページを取得。横断の検索APIは可視条件付きの候補検索で上限8件。本人のピックアップを保存するモデルはない。案件ダッシュボードは進捗・残ポイント等を表示する。[個人画面](../frontend/src/pages/DashboardPage.tsx)・[案件画面](../frontend/src/pages/ProjectDashboardPage.tsx)・[検索](../backend/src/tickets/handlers.rs)・[DDL](../backend/migrations/)
- **時間と履歴。** `work_sessions` は利用者の開始・終了区間を持ち、部分一意索引が未終了を1人1件に制限する。稼働の本人訂正、日報の承認・月締めによる編集制限がある。日報には活動ログから当日の操作票を集める。指定承認者による月締め・解除は操作履歴に残る。これらと独立に `actual-duration` は状態履歴から「進行中に滞在した秒数」を再構成する。複数人の実作業時間も、中断して放置した時間も区別できない。[稼働](../backend/src/work_sessions/handlers.rs)・[日報](../backend/src/daily_reports/repo.rs)・[承認と解除](../backend/src/approvals/handlers.rs)・[経過時間](../backend/src/tickets/duration.rs)

## 認証と可視範囲

Cookieセッションでは変更系リクエストにCSRFトークンを照合する。Bearer認証ではAPIトークンのスコープと利用者の有効性を確認する。サイト権限は admin/general。案件では viewer が閲覧、member が編集、manager が管理可能。サイト管理者には所属の例外があり、アーカイブ済み案件の内容への書込みは禁止。**現行に限りメンバーなし案件は認証済み利用者に閲覧・編集を許す。** 稼働承認は対象者との承認者関係で判定する。[認証extractor](../backend/src/auth/extractors.rs)・[APIトークン](../backend/src/auth/api_tokens.rs)・[案件認可](../backend/src/access.rs)・[承認](../backend/src/approvals/handlers.rs)

## 設計レビューで判明したリスクと対処案

| コードで確認したこと | 生じ得る問題 | 推奨する対処（未実装） |
|---|---|---|
| 案件作成と初期所属追加は別SQL。最後のmanagerの確認と所属更新も別操作。[作成](../backend/src/projects/repo.rs)・[所属変更](../backend/src/project_members/handlers.rs) | 途中失敗や競合で責任者を失う。0人なら公開範囲も変わる。 | 案件・責任者を一括確定し、同じ案件の責任者変更を直列化。旧データの扱いはQ1。 |
| 親・マイルストーンの外部キーは存在を保証するが同一案件まで保証しない。依存関係操作の認可は起点票中心。[DDL](../backend/migrations/20250703000001_init.sql)・[親子操作](../backend/src/tickets/repo.rs)・[依存操作](../backend/src/tickets/dependencies.rs) | ツリー表示・削除が案件の境界を越え得る。関連先の情報や操作が意図した共有範囲から外れる。 | 関係の両端と操作対象の所属を検証。越境依存は親子と分けて扱い、検索・日報・通知等も同じ可視性で点検。 |
| チケットの更新は旧値を読んで全項目を書き戻し、活動履歴はその後に記録。[更新SQL](../backend/src/tickets/repo.rs)・[履歴記録](../backend/src/tickets/handlers.rs) | 同時編集の消失、履歴と現在値の不一致。履歴を使う集計にも波及。 | 更新・履歴を同じトランザクションにし、古い版からの操作を検出する。 |
| 案件ごとの全件取得に加え、通常の票の読み込みは子孫数を再帰集計。[個人画面](../frontend/src/pages/DashboardPage.tsx)・[取得SQL](../backend/src/tickets/repo.rs) | 案件数・ツリー深度が増えると横断画面の負荷が増す。 | 可視性付きの軽い一覧をサーバーでページング。索引追加は代表データで実行計画を確認して判断。 |

既存データの件数・整合性、並行操作、深いWBSでの負荷は対象の改修時に検証する。採用する設計と作業順は[要求台帳](REQUIREMENTS.md)と[追跡Issue #2](https://github.com/jo7ueb/neport/issues/2)を参照。
