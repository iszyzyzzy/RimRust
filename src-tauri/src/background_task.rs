use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc};
use tauri::{AppHandle, Emitter};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{debug, info, span, warn};
use std::time::Duration;
use tokio::time::timeout;
use dashmap::{DashMap, DashSet};
use tracing::Instrument;

pub type TaskFn = Box<
    dyn FnOnce(
            Arc<AppHandle>,
            TaskStatusType,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send,
>;

pub struct Task {
    id: String,
    name: String,
    task: Option<TaskFn>,
    status: Arc<Mutex<TaskStatusAdd>>,
    wait_for: Vec<String>,

}

impl Task {
    pub async fn new(name: String, task: TaskFn, app: Arc<AppHandle>,wait_for: Vec<String>,status_tx: mpsc::Sender<TaskStatusUpdate>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id: id.clone(),
            name: name.clone(),
            task: Some(task),
            status: TaskStatusAdd::new(
                app,
                TaskStatus {
                    id,
                    name,
                    status: "刚创建".to_string(),
                    info: "".to_string(),
                    progress: 0.0,
                },
                status_tx,
            ).await,
            wait_for,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskStatusUpdate {
    New(TaskStatus),
    Status(String, String),  // (task_id, status)
    Info(String, String),    // (task_id, info)
    Progress(String, f64),   // (task_id, progress)
}

pub struct TaskManager {
    sender: mpsc::Sender<Task>,
    app: Arc<AppHandle>,
    running_tasks: Arc<DashMap<String, broadcast::Sender<()>>>,
    _finished_tasks: Arc<DashSet<String>>,
    status_tx: mpsc::Sender<TaskStatusUpdate>,
    _runtime: Arc<tokio::runtime::Runtime>,
}

impl TaskManager {
    pub fn new(app: AppHandle, backgroud_worker: usize) -> Self {
        info!("后台任务管理器初始化");

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .thread_name("task-manager-worker")
            .enable_all()
            .worker_threads(backgroud_worker)
            .build()
            .expect("Failed to create task manager runtime");
        let runtime = Arc::new(runtime);

        let (sender, mut receiver) = mpsc::channel::<Task>(50000);
        let (status_tx, mut status_rx) = mpsc::channel::<TaskStatusUpdate>(50000);
        let app = Arc::new(app);
        let app_clone = app.clone();
        let running_tasks: Arc<DashMap<String, broadcast::Sender<()>>> = Arc::new(DashMap::new());
        let running_tasks_clone = running_tasks.clone();
        let finished_tasks = Arc::new(DashSet::new());
        let finished_tasks_clone = finished_tasks.clone();

        // 启动状态更新聚合循环
        let status_app = app_clone.clone();
        tokio::spawn(async move {
            let mut updates = VecDeque::new();
            let mut interval = tokio::time::interval(Duration::from_millis(80));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            
            loop {
                // 等待下一个固定时间点
                interval.tick().await;
                debug!("任务状态更新聚合循环");
                
                // 收集这段时间内的所有更新
                while let Ok(update) = status_rx.try_recv() {
                    debug!(update = ?update, "收到任务状态更新");
                    updates.push_back(update);
                }
                
                // 发送并清理
                if !updates.is_empty() {
                    debug!(updates = ?updates, "发送任务状态更新");
                    match timeout(Duration::from_secs(1), async {status_app.emit("task_status_update_many", updates.clone())}).await {
                        Ok(Ok(_)) => {
                            info!("成功发送任务状态更新 len={}", updates.len());
                            updates.clear();
                        },
                        Ok(Err(e)) => {
                            warn!("emit失败: {}", e);
                        }
                        Err(e) => {
                            warn!("emit超时: {}", e);
                        }
                    }
                }
            }
        });

        // 启动任务处理循环
        let runtime_clone = runtime.clone();
        tokio::spawn(async move {
            info!("后台任务处理循环启动");
            let runtime = runtime_clone;

            while let Some(task) = receiver.recv().await {
                //let sem = sem_clone.clone().acquire_owned().await.unwrap();
                let app = app_clone.clone();
                let running_tasks = running_tasks_clone.clone();
                let finished_tasks = finished_tasks_clone.clone();
                
                if let Some(task_fn) = task.task {
                    let task_id = task.status.lock().await.get_task_id();
                    let span = span!(tracing::Level::INFO, "task", task_id);
                    runtime.spawn(async move {

/*                         let (complete_tx, _) = broadcast::channel(1);
                        running_tasks.insert(task.id.clone(), complete_tx.clone()); */

                        let mut txs: Vec<_> = Vec::new();
                        for wait_for in task.wait_for.iter() {
                            if let Some(tx) = running_tasks.get(wait_for) {
                                txs.push((wait_for.clone(), tx.subscribe()));
                            }
                        }
                        
                        match timeout(Duration::from_secs(120), async {
                            for (id, mut rx) in txs {
                                if !finished_tasks.contains(&id) {
                                    debug!(wait_for = id, "等待任务完成");
                                    task.status.lock().await.update_status("等待中");
                                    let _ = rx.recv().await;
                                }
                            }
                        }).await {
                            Ok(_) => (),
                            Err(_) => {
                                warn!("等待依赖任务超时");
                                task.status.lock().await.error("等待依赖任务超时(2分钟)");
                                return;
                            }
                        }
                        
                        info!(task_name = task.name, "执行任务");
                        //let _permit = sem;
                        task.status.lock().await.update_status("运行中");

                        let res = task_fn(app, task.status.clone()).await;
                        
                        if let Err(e) = res {
                            warn!(
                                task_name = task.name,
                                "任务出错: {}",
                                e
                            );
                            task.status.lock().await.error(e);
                        } else {
                            task.status.lock().await.finish();
                        }

                        running_tasks.get(&task.id).map(|tx| {
                            let _ = tx.send(());
                        });
                        running_tasks.remove(&task.id);
                        finished_tasks.insert(task.id);
                    }.instrument(span));
                }
            }
        });

        Self {
            sender,
            app,
            running_tasks,
            status_tx,
            _finished_tasks: finished_tasks,
            _runtime: runtime,
        }
    }

    pub fn get_status_tx(&self) -> mpsc::Sender<TaskStatusUpdate> {
        self.status_tx.clone()
    }

    fn wrap_timeout_task(&self, task: TaskFn, timeout_secs: Option<u64>) -> TaskFn {
        if let Some(timeout_secs) = timeout_secs {
            Box::new(move |app: Arc<AppHandle>, status: TaskStatusType| {
                Box::pin(async move {
                    match timeout(Duration::from_secs(timeout_secs), task(app, status.clone())).await {
                        Ok(result) => result,
                        Err(_) => {
                            status.lock().await.timeout(timeout_secs);
//                            warn!("任务超时\n{:#?}", std::backtrace::Backtrace::force_capture());
                            Err("任务超时".to_string())
                        }
                    }
                }) as Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
            })
        } else {
            task
        }
    }

    pub async fn add_task(&self, name: String, task: TaskFn, timeout_secs: Option<u64>, wait_for: Option<Vec<String>>) -> String {
        let task = self.wrap_timeout_task(task, timeout_secs);
        let task = Task::new(name, task, self.app.clone(), wait_for.unwrap_or_default(), self.status_tx.clone()).await;
        info!(task_name = task.name, task_id = task.id, "创建任务结构体");
        let task_id = task.id.clone();
        let (complete_tx, _) = broadcast::channel(1);
        self.running_tasks.insert(task_id.clone(), complete_tx);
        debug!("发送任务: {}", task_id);
        self.sender
            .send(task)
            .await
            .unwrap();
        task_id
    }

    pub async fn wait_for_task(&self, task_id: &String) {
        if let Some(tx) = self.running_tasks.get(task_id) {
            let _ = tx.subscribe().recv().await;
        }
    }

}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub info: String,
    pub progress: f64,
}

pub type TaskStatusType = Arc<Mutex<TaskStatusAdd>>;

pub struct TaskStatusAdd {
    status: Option<TaskStatus>,
    virtual_: bool,
    status_tx: Option<mpsc::Sender<TaskStatusUpdate>>,
}

impl std::fmt::Debug for TaskStatusAdd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskStatusAdd")
            .field("status", &self.status)
            .finish()
    }
}

impl TaskStatusAdd {
    // #[instrument(skip(app))]
    pub async fn new(app: Arc<AppHandle>, status: TaskStatus, status_tx: mpsc::Sender<TaskStatusUpdate>) -> Arc<Mutex<Self>> {
        status_tx.send(TaskStatusUpdate::New(status.clone())).await.unwrap();
        Arc::new(Mutex::new(Self { 
            status: Some(status),
            virtual_: false,
            status_tx: Some(status_tx),
        }))
    }
    pub fn _new_virtual() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            status:None,
            virtual_: true,
            status_tx: None,
        }))
    }
    pub fn get_task_id(&self) -> String {
        if self.virtual_ {
            return "virtual".to_string();
        }
        self.status.as_ref().unwrap().id.clone()
    }
    pub fn update_status(&mut self, status: impl Into<String>) {
        if self.virtual_ {
            return;
        }
        let status = status.into();
        self.status.as_mut().unwrap().status = status.clone();
        if let Some(tx) = &self.status_tx {
            let _ = tx.try_send(TaskStatusUpdate::Status(
                self.status.as_ref().unwrap().id.clone(),
                status
            ));
        }
    }
    pub fn update_info(&mut self, info: impl Into<String>) {
        if self.virtual_ {
            return;
        }
        
        let info = info.into();
        self.status.as_mut().unwrap().info = info.clone();
        if let Some(tx) = &self.status_tx {
            let _ = tx.try_send(TaskStatusUpdate::Info(
                self.status.as_ref().unwrap().id.clone(),
                info
            ));
        }
    }
    pub fn update_progress(&mut self, progress: f64) {
        if self.virtual_ {
            return;
        }
        self.status.as_mut().unwrap().progress = progress;
        if let Some(tx) = &self.status_tx {
            let _ = tx.try_send(TaskStatusUpdate::Progress(
                self.status.as_ref().unwrap().id.clone(),
                progress
            ));
        }
    }
    pub fn timeout(&mut self, sec: u64) {
        if self.virtual_ {
            return;
        }
        self.update_status("超时");
        self.update_progress(0.0);
        warn!(task_id = ?self.status.as_ref().unwrap().id,"任务执行超过{}秒", sec);
    }
    pub fn error(&mut self, error: impl Into<String>) {
        if self.virtual_ {
            return;
        }
        self.update_status("出错");
        self.update_info(error);
    }
    pub fn finish(&mut self) {
        if self.virtual_ {
            return;
        }
        self.update_info("结束");
        self.update_status("已完成");
        self.update_progress(100.0);
    }
}

impl Drop for TaskStatusAdd {
    fn drop(&mut self) {
        if self.virtual_ {
            return;
        }
        info!(
            task_name = self.status.as_ref().unwrap().name,
            task_id = self.status.as_ref().unwrap().id,
            "任务结束"
        );
        //self.update_status("已结束");
    }
}
