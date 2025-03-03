use clap::Parser;
use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;
use termion::color;

const URL: &str = "https://api.siliconflow.cn/v1/chat/completions";
const MODEL: &str = "deepseek-ai/DeepSeek-R1-Distill-Llama-8B";

/// Config for your ag
#[derive(Parser)]
struct Cli {
    /// Support multiple lines input or not
    #[arg(short, long)]
    multi_lines: bool,

    /// Choice of your model. DeepSeek-R1-Distill-Llama-8B default, which is free.
    /// r1: deepseek-ai/DeepSeek-R1
    #[arg(long)]
    model: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Payload {
    messages: Vec<HashMap<String, String>>,
    model: String,
    stream: bool,
}

impl Payload {
    fn new(model: String, stream: bool) -> Payload {
        Payload {
            model,
            stream,
            messages: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Chunk {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Serialize, Deserialize, Debug)]
struct Usage {
    total_tokens: usize,
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
    let cli = Cli::parse();
    let api_key = std::env::var("API_KEY").expect("You must set the API_KEY environment variable");
    let client = reqwest::Client::new();
    let mut payload = Payload::new(
        if let Some(model) = cli.model {
            if &model == "r1" {
                "deepseek-ai/DeepSeek-R1".into()
            } else {
                panic!("Unknown Model");
            }
        } else {
            MODEL.into()
        },
        true,
    );
    loop {
        print!("☁️ : ");
        std::io::stdout().flush().unwrap();
        let mut s = String::new();
        if let Err(error) = {
            if cli.multi_lines {
                std::io::stdin().read_to_string(&mut s)
            } else {
                std::io::stdin().read_line(&mut s)
            }
        } {
            println!("❌ Read input error: {}", error);
            continue;
        };
        let s = s.trim();
        if s == "q" || s.is_empty() {
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
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .expect("Failed to send request. Check your Internet connection")
            .bytes_stream();
        print!(
            "🚀 {} 深度思考中...\n{}<think>\n",
            payload.model,
            color::Fg(color::Rgb(200, 200, 200))
        );
        let mut think = true;
        std::io::stdout().flush().unwrap();
        let mut tokens = 0;
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                continue;
            }
            let chunk = chunk.unwrap().slice(6..);
            if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk) {
                tokens = chunk.usage.total_tokens;
                if let Some(content) = chunk.choices[0].clone().delta.content {
                    if think {
                        println!("\n</think>\n{}", color::Fg(color::Reset));
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
        println!("\n{}total tokens: {}{}", color::Fg(color::Yellow),tokens, color::Fg(color::Reset));
    }
}
