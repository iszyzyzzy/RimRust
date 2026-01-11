
use std::{
    collections::HashMap, sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    }, time::Duration
};
use tantivy::{
    doc, query::QueryParser, schema::*, Index, IndexReader, IndexWriter,
    ReloadPolicy,
};
use tauri::{AppHandle, Manager};
use tokio::{
    sync::{oneshot, Mutex, RwLock},
    task::JoinHandle,
    time::Instant,
};
use tracing::{info, warn};

use super::super::{SearchField, SearchResult, SearchResultItem};
use super::{collector, tokenizer};
use crate::{
    background_task::{self, TaskStatus, TaskStatusAdd},
    mods::base_list::*,
    types::*,
};

use tokio::sync::mpsc;

enum WriteOp {
    Add(ModInner),
    Remove(Id),
    Update(ModInner),
}

pub struct SearchDataInner {
    mod_index: Index,
    mod_writer: Arc<RwLock<IndexWriter>>,
    mod_reader: IndexReader,
    last_commit: Arc<Mutex<Instant>>,
    uncommitted_docs: Arc<AtomicU64>,
    commit_task: Arc<Mutex<Option<(oneshot::Sender<()>, JoinHandle<()>)>>>,
    game_version: Version,
    write_tx: mpsc::Sender<WriteOp>,
    field_map: HashMap<SearchField, Field>,
    field_map_reverse: HashMap<Field, SearchField>,
}

impl Default for SearchDataInner {
    fn default() -> Self {
        Self::new("", Version::default())
    }
}

impl SearchDataInner {
    const COMMIT_THRESHOLD: u64 = 10;
    const COMMIT_INTERVAL: Duration = Duration::from_secs(10);

    pub fn new(app_data_path: &str, game_version: Version) -> Self {
        info!(app_data_path = ?app_data_path, game_version = ?game_version,"初始化搜索引擎");
        let text_option = TextOptions::default().set_stored().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("mix")
                .set_index_option(IndexRecordOption::WithFreqs),
        );
        let mut schema_builder = Schema::builder();
        let field_id = schema_builder.add_text_field("id", STRING | FAST | STORED);
        let field_name = schema_builder.add_text_field("name", text_option.clone());
        let field_description = schema_builder.add_text_field("description", text_option.clone());
        let field_author = schema_builder.add_text_field("author", text_option.clone());
        let field_package_id = schema_builder.add_text_field("packageId", text_option.clone());
        let field_display_name = schema_builder.add_text_field("display_name", text_option.clone());
        let mod_schema = schema_builder.build();
        // 这里不用STORED的话，做matched_fields就得从外面的hashmap里取
        // 两者大概也就 2ms -> 3ms, 5mb -> 8mb 的区别, 我懒得再传入mods了
        //let tokenizer = tantivy_jieba::JiebaTokenizer {};
        /*         let mod_index = if !(app_data_path == "") {
            if !std::fs::exists(format!("{}\\mod_search_index", app_data_path)).unwrap() {
                std::fs::create_dir_all(format!("{}\\mod_search_index", app_data_path)).unwrap();
            }
            if cfg!(debug_assertions) { // dev下有可能会残留.lock文件导致后面添加之类的操作失败
                std::fs::remove_dir_all(format!("{}\\mod_search_index", app_data_path)).unwrap();
                std::fs::create_dir_all(format!("{}\\mod_search_index", app_data_path)).unwrap();
            }
            Index::open_in_dir(format!("{}\\mod_search_index", app_data_path)).unwrap_or_else(|_| {
                let index = Index::create_in_dir(
                    format!("{}\\mod_search_index", app_data_path),
                    mod_schema.clone(),
                )
                .unwrap();
                index.tokenizers().register("jieba", tokenizer);
                index
            })
        } else {
            let index = Index::create_in_ram(mod_schema.clone());
            index.tokenizers().register("jieba", tokenizer);
            index
        }; */
        // 既然我每次启动都有一遍全部add的流程，不如直接create_in_ram
        // 我测试了一下，占用大概从3mb -> 5mb，不是很大
        let mod_index = Index::create_in_ram(mod_schema.clone());
        //mod_index.tokenizers().register("jieba", tokenizer);
        let tokenizer = tokenizer::MixedTokenizer::new();
        mod_index.tokenizers().register("mix", tokenizer);

        let mod_writer = Arc::new(RwLock::new(mod_index.writer(50_000_000).unwrap()));
        let mod_reader = mod_index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .unwrap();

        let uncommitted_docs = Arc::new(AtomicU64::new(0));

        let (write_tx, mut write_rx) = mpsc::channel(1000);

        let mod_writer_clone = mod_writer.clone();
        let uncommitted_docs_clone = uncommitted_docs.clone();
        let schema_clone = mod_index.schema();
        let game_version_clone = game_version.clone();
        let write_tx_clone = write_tx.clone();

        tokio::task::spawn(async move {
            let mod_writer = mod_writer_clone;
            let uncommitted_docs = uncommitted_docs_clone;
            let schema = schema_clone;
            let game_version = game_version_clone;
            let write_tx = write_tx_clone;

            let id = schema.get_field("id").unwrap();
            let name = schema.get_field("name").unwrap();
            let description = schema.get_field("description").unwrap();
            let author = schema.get_field("author").unwrap();
            let package_id = schema.get_field("packageId").unwrap();
            let display_name = schema.get_field("display_name").unwrap();

            while let Some(op) = write_rx.recv().await {
                match op {
                    WriteOp::Add(mod_) => {
                        let doc = doc!(
                            id => mod_.id.to_string(),
                            name => mod_.name.clone(),
                            description => mod_
                                .description
                                .get(&game_version)
                                .unwrap_or(&"".to_string())
                                .clone(),
                            author => mod_.author.clone(),
                            package_id => mod_.package_id.to_string(),
                            display_name => mod_.display_name.clone()
                        );
                        let writer = mod_writer.write().await;
                        if let Err(e) = writer.add_document(doc) {
                            warn!(
                                "添加mod到搜索引擎失败: name:{} err: {} ,重新加入队列",
                                mod_.name, e
                            );
                            write_tx.send(WriteOp::Add(mod_)).await.unwrap();
                        } else {
                            uncommitted_docs.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    WriteOp::Remove(target_id) => {
                        let term = Term::from_field_text(id, &target_id.to_string());
                        let writer = mod_writer.write().await;
                        writer.delete_term(term);
                        uncommitted_docs.fetch_add(1, Ordering::Relaxed);
                    }
                    WriteOp::Update(mod_) => {
                        let term = Term::from_field_text(id, &mod_.id.to_string());
                        let writer = mod_writer.write().await;
                        writer.delete_term(term);
                        drop(writer);
                        let doc = doc!(
                            id => mod_.id.to_string(),
                            name => mod_.name.clone(),
                            description => mod_
                                .description
                                .get(&game_version)
                                .unwrap_or(&"".to_string())
                                .clone(),
                            author => mod_.author.clone(),
                            package_id => mod_.package_id.to_string(),
                            display_name => mod_.display_name.clone()
                        );
                        let writer = mod_writer.write().await;
                        if let Err(e) = writer.add_document(doc) {
                            warn!(
                                "添加mod到搜索引擎失败: name:{} err: {} ,重新加入队列",
                                mod_.name, e
                            );
                            write_tx.send(WriteOp::Add(mod_)).await.unwrap();
                        } else {
                            uncommitted_docs.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        });

        let t_map = [
            (SearchField::Id, field_id),
            (SearchField::Name, field_name),
            (SearchField::Description, field_description),
            (SearchField::Author, field_author),
            (SearchField::PackageId, field_package_id),
            (SearchField::DisplayName, field_display_name),
        ];

        let searcher = Self {
            mod_index,
            mod_writer,
            mod_reader,
            last_commit: Arc::new(Mutex::new(Instant::now())),
            uncommitted_docs,
            commit_task: Arc::new(Mutex::new(None)),
            game_version,
            write_tx: write_tx,
            field_map: t_map.iter().cloned().collect(),
            field_map_reverse: t_map.iter().cloned().map(|(k, v)| (v, k)).collect(),
        };
        searcher
    }
    async fn force_commit(&self) -> Result<(), tantivy::TantivyError> {
        info!("强制提交");
        let mut writer = self.mod_writer.write().await;
        writer.commit()?;
        self.uncommitted_docs.store(0, Ordering::Relaxed);
        *self.last_commit.lock().await = Instant::now();
        Ok(())
    }
    pub async fn start_auto_commit(&mut self, app: AppHandle) {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let mod_writer = self.mod_writer.clone();
        let last_commit = self.last_commit.clone();
        let uncommitted_docs = self.uncommitted_docs.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(SearchDataInner::COMMIT_INTERVAL / 2);
            let mut shutdown_rx = shutdown_rx;
            let app = Arc::new(app);
            let task_manager = app.state::<Mutex<background_task::TaskManager>>();
            let task_manager = task_manager.lock().await;
            let status_tx = task_manager.get_status_tx();
            drop(task_manager);

            let task_status = TaskStatusAdd::new(
                app,
                TaskStatus {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "索引构建".to_string(),
                    status: "运行中".to_string(),
                    info: "".to_string(),
                    progress: 0.0,
                },
                status_tx,
            ).await;
            let mut task_status = task_status.lock().await;
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        if let Err(e) = async {
                            let mut writer = mod_writer.write().await;
                            writer.commit()?;
                            uncommitted_docs.store(0, Ordering::Relaxed);
                            *last_commit.lock().await = Instant::now();
                            Ok::<(), tantivy::TantivyError>(())
                        }.await {
                            warn!("自动提交失败: {}", e);
                        }
                        task_status.update_status("已结束");
                        break;
                    }
                    _ = interval.tick() => {
                        let mut retry_count = 0;
                        const MAX_RETRIES: u32 = 3;
                        while retry_count < MAX_RETRIES {
                            match tokio::time::timeout(
                                Duration::from_secs(30),
                                async {
                                    let uncommitted = uncommitted_docs.load(Ordering::Relaxed);
                                    if uncommitted > 0 {
                                        info!("自动提交: {}", uncommitted);
                                        task_status.update_status("运行中");
                                        task_status.update_info(format!("正在构建{}个文档索引", uncommitted));
                                        task_status.update_progress(0.0);
                                        if let Err(e) = async {
                                            let mut writer = mod_writer.write().await;
                                            writer.commit()?;
                                            uncommitted_docs.store(0, Ordering::Relaxed);
                                            *last_commit.lock().await = Instant::now();
                                            Ok::<(), tantivy::TantivyError>(())
                                        }.await {
                                            warn!("自动提交失败: {}", e);
                                        } else {
                                            info!("自动提交成功");
                                            task_status.update_info("索引完成");
                                            task_status.update_progress(100.0);
                                            task_status.update_status("休眠中");
                                        }
                                    }
                                },
                            ).await {
                                Ok(_) => break,
                                Err(_) => {
                                    retry_count += 1;
                                    warn!("自动提交超时，重试第{}次", retry_count);
                                }
                            }
                        }
                        if retry_count == MAX_RETRIES {
                            warn!("自动提交失败");
                        }
                    }
                }
            }
        });
        *self.commit_task.lock().await = Some((shutdown_tx, handle));
    }
    pub async fn stop_auto_commit(&self) {
        if let Some((tx, handle)) = self.commit_task.lock().await.take() {
            tx.send(()).unwrap();
            handle.await.unwrap();
        }
    }
    pub async fn add(&self, mod_: &ModInner) {
        self.write_tx
            .send(WriteOp::Add(mod_.clone()))
            .await
            .unwrap();
    }
    pub async fn search(&self, query_text: &str, search_field: Vec<SearchField>, enabled_only: bool) -> SearchResult {
        //if self.uncommitted_docs.load(Ordering::Relaxed) > Self::COMMIT_THRESHOLD {
        //    if let Err(e) = self.force_commit().await {
        //        warn!("强制提交失败: {}", e);
        //    }
        //}
        let searcher = self.mod_reader.searcher();
        let schema = self.mod_index.schema();
        let fields = SearchField::to_str_vec(&search_field)
            .iter()
            .map(|f| schema.get_field(f).unwrap())
            .collect::<Vec<Field>>();
        let query_parser = QueryParser::for_index(&self.mod_index, fields.clone());
        let query = query_parser.parse_query(query_text).unwrap();
        /*        let top_docs = searcher
             .search(&query, &TopDocs::with_limit(1000))
             .unwrap();
         let mut mods = Vec::new();
         for (_score, doc_address) in top_docs {
             let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
             let id = retrieved_doc
                 .get_first(schema.get_field("id").unwrap())
                 .unwrap()
                 .as_str()
                 .unwrap()
                 .to_string();
             mods.push(Id::from_str(id));
         }
        // TODO 写一个完整的matched_fields得写自定义collector，
        // TODO 还得自己写tokenizer来做词形还原和前缀匹配，写不动了，就这样吧
         for (_score, doc_address) in top_docs {
             let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
             let id = retrieved_doc.get_first(schema.get_field("id").unwrap()).and_then(|v| v.as_str()).unwrap().to_string();
             let matched_fields = search_field.iter().filter_map(|f| {
                 let field = schema.get_field(f.to_str()).unwrap();
                 let field_value = retrieved_doc.get_first(field).and_then(|v| v.as_str()).unwrap_or("");
                 if field_value.to_lowercase().contains(&query_text.to_lowercase()) {
                     Some(f.to_string())
                 } else {
                     None
                 }
             }).collect();
             let score = _score;
             mods.push(SearchResultItem {
                 id,
                 matched_fields,
                 score,
             });
         }
         */

        let mut tokenizer = self.mod_index.tokenizers().get("mix").unwrap();
        let mut token_stream = tokenizer.token_stream(query_text);
        let mut term = Vec::new();
        while token_stream.advance() {
            let token = token_stream.token();
            term.push(token.text.clone());
        }

        let mut collector = collector::MatchedFieldsCollector::new(1000, fields, term.clone());
        let doc_set = searcher.search(&query, &mut collector).unwrap();

        let mut mods = Vec::new();
        for (score, _, id, matched_fields) in doc_set {
            /*             let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
            let id = retrieved_doc
                .get_first(schema.get_field("id").unwrap())
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(); */
            mods.push(SearchResultItem {
                id,
                matched_fields: matched_fields.iter().map(|f| self.field_map_reverse.get(f).unwrap().to_string()).collect(),
                score,
            });
        }

        SearchResult {
            total: mods.len(),
            mods,
            //hightlight: format!("({})", pattern),
            highlight: term,
        }
    }
    pub async fn remove(&self, id: &Id) {
        self.write_tx
            .send(WriteOp::Remove(id.clone()))
            .await
            .unwrap();
    }

    pub async fn update(&self, mod_: &ModInner) {
        self.write_tx
            .send(WriteOp::Update(mod_.clone()))
            .await
            .unwrap();
    }
}

impl Drop for SearchDataInner {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                self.stop_auto_commit().await;
                self.force_commit().await.unwrap();
            });
        })
    }
}
