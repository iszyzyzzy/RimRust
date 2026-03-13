use ahash::{HashMap, RandomState};
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Clone, bincode::Decode, bincode::Encode)]
pub struct AutoTranslateResult {
    pub code: i32,
    pub message: Option<String>,
    pub data: String,
    pub source: String,
    pub target: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum AutoTranslateEvent {
    Processing {},
    PartialResult { content: String },
    Completed { result: AutoTranslateResult },
    Error { message: String },
}

#[derive(Debug, Deserialize)]
struct LlmChatCompletionResponse {
    choices: Vec<LlmChoice>,
}

#[derive(Debug, Deserialize)]
struct LlmChoice {
    message: LlmMessage,
}

#[derive(Debug, Deserialize)]
struct LlmMessage {
    content: String,
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

async fn llm_translate(
    text: &str,
    from: &str,
    to: &str,
    proxy: Option<String>,
    api_entry_point: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    let api_entry_point = normalize_optional_string(api_entry_point)
        .ok_or_else(|| "未配置 llm ApiEntryPoint".to_string())?;
    let api_key = normalize_optional_string(api_key)
        .ok_or_else(|| "未配置 llm api_key".to_string())?;

    let endpoint = if api_entry_point.ends_with("/chat/completions") {
        api_entry_point
    } else {
        format!("{}/chat/completions", api_entry_point.trim_end_matches('/'))
    };

    let mut client_builder = reqwest::Client::builder();
    if let Some(proxy) = normalize_optional_string(proxy) {
        let proxy = reqwest::Proxy::all(&proxy).map_err(|e| e.to_string())?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build().map_err(|e| e.to_string())?;

    let request_body = serde_json::json!({
        "model": model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        "messages": [
            {
                "role": "system",
                "content": "你是一个翻译助手。请严格翻译用户文本，不要解释，不要添加额外内容，格式标记，图片等信息原样保留即可。"
            },
            {
                "role": "system",
                "content": "请注意，你正在翻译游戏RimWorld的mod相关文本，此信息作为背景信息提供，但不要在翻译时添加任何与此相关的额外信息。"
            },
            {
                "role": "user",
                "content": format!("请将以下文本从 {} 翻译为 {}：\n{}", from, to, text)
            }
        ],
        "temperature": 1
    });

    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM 翻译请求失败({}): {}", status.as_u16(), err_text));
    }

    let response_body = response
        .json::<LlmChatCompletionResponse>()
        .await
        .map_err(|e| e.to_string())?;
    let translated = response_body
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "LLM 返回结果为空".to_string())?;

    Ok(translated)
}

async fn llm_translate_streaming(
    text: &str,
    from: &str,
    to: &str,
    proxy: Option<String>,
    api_entry_point: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    event_tx: &tokio::sync::broadcast::Sender<AutoTranslateEvent>,
) -> Result<String, String> {
    let api_entry_point = normalize_optional_string(api_entry_point)
        .ok_or_else(|| "未配置 llm ApiEntryPoint".to_string())?;
    let api_key = normalize_optional_string(api_key)
        .ok_or_else(|| "未配置 llm api_key".to_string())?;

    let endpoint = if api_entry_point.ends_with("/chat/completions") {
        api_entry_point
    } else {
        format!("{}/chat/completions", api_entry_point.trim_end_matches('/'))
    };

    let mut client_builder = reqwest::Client::builder();
    if let Some(proxy) = normalize_optional_string(proxy) {
        let proxy = reqwest::Proxy::all(&proxy).map_err(|e| e.to_string())?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder.build().map_err(|e| e.to_string())?;

    let request_body = serde_json::json!({
        "model": model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        "messages": [
            {
                "role": "system",
                "content": "你是一个翻译助手。请严格翻译用户文本，不要解释，不要添加额外内容，格式标记，图片等信息原样保留即可。"
            },
            {
                "role": "system",
                "content": "请注意，你正在翻译游戏RimWorld的mod相关文本，此信息作为背景信息提供，但不要在翻译时添加任何与此相关的额外信息。"
            },
            {
                "role": "user",
                "content": format!("请将以下文本从 {} 翻译为 {}：\n{}", from, to, text)
            }
        ],
        "temperature": 1,
        "stream": true
    });

    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM 流式翻译请求失败({}): {}", status.as_u16(), err_text));
    }

    let mut full_content = String::new();
    let mut stream = response.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        let event = event.map_err(|e| e.to_string())?;
        if event.data == "[DONE]" {
            break;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&event.data) {
            if let Some(delta) = json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
            {
                full_content.push_str(delta);
                debug!("收到流式内容: {}", delta);
                let _ = event_tx.send(AutoTranslateEvent::PartialResult {
                    content: delta.to_string(),
                });
            }
        }
    }

    if full_content.trim().is_empty() {
        return Err("LLM 返回结果为空".to_string());
    }

    Ok(full_content)
}

pub async fn auto_translate(
    text: String,
    from: String,
    to: String,
    proxy: Option<String>,
    api_entry_point: Option<String>,
    api_key: Option<String>,
    api_model: Option<String>,
    cache: Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    ongoing_auto_translate: Arc<
        Mutex<
            HashMap<
                String,
                Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult, String>>>,
            >,
        >,
    >,
) -> Result<AutoTranslateResult, String> {
    if cache.lock().unwrap().contains(&text) {
        debug!("翻译缓存命中");
        return Ok(cache.lock().unwrap().get(&text).unwrap().clone());
    }

    let rx = {
        let mut ongoing = ongoing_auto_translate.lock().unwrap();
        if ongoing.contains_key(&text) {
            debug!("翻译正在进行中");
            let (tx, rx) = tokio::sync::oneshot::channel();
            ongoing.get_mut(&text).unwrap().push(tx);
            Some(rx)
        } else {
            ongoing.insert(text.clone(), vec![]);
            None
        }
    };

    if let Some(rx) = rx {
        return match rx.await {
            Ok(res) => res,
            Err(e) => Err(e.to_string()),
        };
    }

    let res = llm_translate(&text, &from, &to, proxy, api_entry_point, api_key, api_model)
        .await
        .map(|translated| {
            let result = AutoTranslateResult {
                code: 200,
                message: None,
                data: translated,
                source: from.clone(),
                target: to.clone(),
            };
            cache.lock().unwrap().put(text.clone(), result.clone());
            debug!(?result, "翻译结果");
            result
        });

    let txs = ongoing_auto_translate.lock().unwrap().remove(&text).unwrap();
    for tx in txs {
        let _ = tx.send(res.clone());
    }

    res
}

pub async fn auto_translate_streaming(
    text: String,
    from: String,
    to: String,
    proxy: Option<String>,
    api_entry_point: Option<String>,
    api_key: Option<String>,
    api_model: Option<String>,
    cache: Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    ongoing_auto_translate: Arc<
        Mutex<
            HashMap<
                String,
                Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult, String>>>,
            >,
        >,
    >,
    ongoing_auto_translate_streaming: Arc<
        Mutex<HashMap<String, tokio::sync::broadcast::Sender<AutoTranslateEvent>>>,
    >,
    on_event: Channel<AutoTranslateEvent>,
) {
    let _ = ongoing_auto_translate;

    // 检查缓存
    if let Ok(cache_lock) = cache.lock() {
        if cache_lock.contains(&text) {
            if let Some(result) = cache_lock.peek(&text) {
                debug!("翻译缓存命中");
                let _ = on_event.send(AutoTranslateEvent::Completed { result: result.clone() });
                return;
            }
        }
    }

    let (sender, is_new_task) = {
        let mut ongoing = ongoing_auto_translate_streaming.lock().unwrap();
        if let Some(sender) = ongoing.get(&text) {
            (sender.clone(), false)
        } else {
            let (sender, _) = tokio::sync::broadcast::channel(256);
            ongoing.insert(text.clone(), sender.clone());
            (sender, true)
        }
    };

    if is_new_task {
        let text_clone = text.clone();
        let from_clone = from.clone();
        let to_clone = to.clone();
        let cache_clone = cache.clone();
        let ongoing_streaming_clone = ongoing_auto_translate_streaming.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            let _ = sender_clone.send(AutoTranslateEvent::Processing {});

            let result = llm_translate_streaming(
                &text_clone,
                &from_clone,
                &to_clone,
                proxy,
                api_entry_point,
                api_key,
                api_model,
                &sender_clone,
            )
            .await
            .map(|translated| {
                let result = AutoTranslateResult {
                    code: 200,
                    message: None,
                    data: translated,
                    source: from_clone,
                    target: to_clone,
                };
                if let Ok(mut cache_lock) = cache_clone.lock() {
                    cache_lock.put(text_clone.clone(), result.clone());
                }
                debug!(?result, "翻译结果");
                result
            });

            match result {
                Ok(result) => {
                    let _ = sender_clone.send(AutoTranslateEvent::Completed { result });
                }
                Err(err) => {
                    let _ = sender_clone.send(AutoTranslateEvent::Error { message: err });
                }
            }

            if let Ok(mut ongoing) = ongoing_streaming_clone.lock() {
                ongoing.remove(&text_clone);
            }
        });
    }

    let mut rx = sender.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                let is_terminal = matches!(
                    event,
                    AutoTranslateEvent::Completed { .. } | AutoTranslateEvent::Error { .. }
                );
                let _ = on_event.send(event);
                if is_terminal {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

