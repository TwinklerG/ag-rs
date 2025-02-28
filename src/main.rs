use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

const URL: &str = "https://api.siliconflow.cn/v1/chat/completions";
const MODEL: &str = "deepseek-ai/DeepSeek-R1-Distill-Llama-8B";

#[derive(Serialize, Deserialize)]
struct Payload {
    messages: Vec<HashMap<String, String>>,
    model: String,
    stream: bool,
}

impl Payload {
    fn new(model: &str, stream: bool) -> Payload {
        Payload {
            model: model.to_string(),
            stream,
            messages: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Chunk {
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Choice {
    delta: Delta,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Delta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("API_KEY").expect("You must set the API_KEY environment variable");
    let client = reqwest::Client::new();
    let mut payload = Payload::new(MODEL, true);
    loop {
        print!("☁️ : ");
        std::io::stdout().flush().unwrap();
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).unwrap();
        let s = s.trim();
        if s == "q".to_string() || s.is_empty() {
            break;
        }
        let mut tmp = HashMap::new();
        tmp.insert("role".to_string(), "user".to_string());
        tmp.insert("content".to_string(), s.to_string());
        payload.messages.push(tmp);
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", api_key).parse().unwrap(),
        );
        headers.insert("Content-Type", "application/json".parse().unwrap());
        let mut stream = client
            .post(URL)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .unwrap()
            .bytes_stream();
        print!("🚀 {} 深度思考中...\n<think>\n\x1b[36m", payload.model);
        let mut think = true;
        std::io::stdout().flush().unwrap();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap().slice(6..);
            if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk) {
                if let Some(content) = chunk.choices[0].clone().delta.content {
                    if think {
                        println!("\n</think>\n\x1b[0m");
                        think = false;
                    }
                    print!("{}", content);
                    std::io::stdout().flush().unwrap();
                } else if let Some(reasoning_content) =
                    chunk.choices[0].clone().delta.reasoning_content
                {
                    print!("{}", reasoning_content);
                    std::io::stdout().flush().unwrap();
                }
            }
        }
        println!();
    }
}
