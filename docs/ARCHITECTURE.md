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

## チケットの更新と関連付け

- チケットの作成・更新・削除と依存の追加・削除は、案件行をロックしてから関連先や循環を検証する。チケット更新、アクセス記録、活動履歴、DB通知は同じトランザクションで確定する。[チケット更新](../backend/src/tickets/handlers.rs)・[ロックと階層検証](../backend/src/tickets/repo.rs)・[依存操作](../backend/src/tickets/dependencies.rs)
- 親・マイルストーンの指定先と依存の両端は有効な同一案件内の要素に限る。依存の削除には、URLのチケットがその依存の端点であることも必要。WBSの子孫・削除、依存一覧・ガント、案件・マイルストーン集計も案件内に絞る。[チケットSQL](../backend/src/tickets/repo.rs)・[依存SQL](../backend/src/tickets/dependencies.rs)・[ガント](../backend/src/gantt/repo.rs)・[集計](../backend/src/milestones/repo.rs)
- PATCHで取得時の `updated_at` を `expected_updated_at` として送ると、版が異なる場合は `409 Conflict` を返す。Webの編集フォームと盤面等はこの照合を使う。省略した既存クライアントのPATCHは、ロック取得後の値に適用する。[API定義](../backend/src/api_docs.rs)・[Webの編集フォーム](../frontend/src/pages/TicketEditPage.tsx)

## 設計レビューで判明したリスクと対処案

| コードで確認したこと | 生じ得る問題 | 推奨する対処（未実装） |
|---|---|---|
| 案件と初期所属は同じトランザクションで確定する。作成の監査記録はコミット後に書く。最後のmanagerの確認と所属更新は別操作。[作成](../backend/src/projects/repo.rs)・[監査記録](../backend/src/projects/handlers.rs)・[所属変更](../backend/src/project_members/handlers.rs) | 監査記録が失敗すると、案件は保存されるがAPIはエラーを返す。所属変更が競合すると責任者を失い、所属が0人なら公開範囲も変わる。 | 監査記録の確定境界を検討し、同じ案件の責任者変更を直列化する。旧データの扱いはQ1。 |
| 同一案件の検証はAPIにあり、親・マイルストーンの外部キーは参照先の存在を保証する。[DDL](../backend/migrations/20250703000001_init.sql)・[関連先の検証](../backend/src/tickets/repo.rs) | 旧データや直接のSQL操作では案件をまたぐ参照IDが残り得る。 | 旧データの引継ぎ方針に沿って補正やDB制約を検討する。案件間の依存は[Issue #11](https://github.com/jo7ueb/neport/issues/11)で設計する。 |
| 案件ごとの全件取得に加え、通常の票の読み込みは子孫数を再帰集計。[個人画面](../frontend/src/pages/DashboardPage.tsx)・[取得SQL](../backend/src/tickets/repo.rs) | 案件数・ツリー深度が増えると横断画面の負荷が増す。 | 可視性付きの軽い一覧をサーバーでページング。索引追加は代表データで実行計画を確認して判断。 |

既存データの件数・整合性、並行操作、深いWBSでの負荷は対象の改修時に検証する。採用する設計と作業順は[要求台帳](REQUIREMENTS.md)と[追跡Issue #2](https://github.com/jo7ueb/neport/issues/2)を参照。
