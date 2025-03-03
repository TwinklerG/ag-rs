use clap::Parser;
use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;
use termion::color;

const URL: &str = "https://api.siliconflow.cn/v1/chat/completions";

/// Config for your ag
#[derive(Parser)]
struct Cli {
    /// Support multiple lines input or not
    #[arg(short, long)]
    multi_lines: bool,

    /// Choice of your model. DeepSeek-R1-Distill-Llama-8B default, which is free.
    /// r1: deepseek-ai/DeepSeek-R1
    /// q2_5-7: Qwen2.5-7B-Instruct
    /// default: deepseek-ai/DeepSeek-R1-Distill-Llama-8B
    #[arg(long, verbatim_doc_comment)]
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
    prompt_tokens: usize,
    completion_tokens: usize,
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

macro_rules! model_map {
    ($var:expr) => {
        match $var {
            "r1" => Ok("deepseek-ai/DeepSeek-R1"),
            "q2_5-7" => Ok("Qwen/Qwen2-7B-Instruct"),
            "ds-8" => Ok("deepseek-ai/DeepSeek-R1-Distill-Llama-8B"),
            _ => Err("deepseek-ai/DeepSeek-R1-Distill-Llama-8B"),
        }
    };
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let api_key = std::env::var("API_KEY").expect("You must set the API_KEY environment variable");
    let client = reqwest::Client::new();
    let model = if let Some(model) = cli.model {
        model
    } else {
        "ds-8".to_string()
    };
    let model: Result<&str, &str> = model_map!(&model[..]);
    let model = match model {
        Ok(model) => {
            println!("{}", model);
            model.to_string()
        }
        Err(model) => {
            println!("Unknown model. Using {} by default", model);
            model.to_string()
        }
    };
    let mut payload = Payload::new(model.clone(), true);
    let mut rl = DefaultEditor::new().unwrap();
    loop {
        let mut input = String::new();
        let mut readline = rl.readline("☁️ : ");
        loop {
            match &readline {
                Ok(line) => {
                    input.push_str(line);
                }
                Err(ReadlineError::Interrupted) => {
                    eprintln!("Terminated with CTRL-C");
                    return;
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
            if !cli.multi_lines {
                break;
            }
            readline = rl.readline("");
        }
        let input = input.trim();
        if input == "q" || input.is_empty() {
            break;
        }
        let mut tmp = HashMap::new();
        tmp.insert("role".to_string(), "user".to_string());
        tmp.insert("content".to_string(), input.to_string());
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
        let mut think = false;
        std::io::stdout().flush().unwrap();
        let mut usage = Usage {
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
        };
        println!("🚀 {}", &model);
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                continue;
            }
            let chunk = chunk.unwrap().slice(6..);
            if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk) {
                usage = chunk.usage;
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
                    if !think {
                        think = true;
                        println!("{}<think>", color::Fg(color::Rgb(200, 200, 200)));
                    }
                    print!("{}", reasoning_content);
                    std::io::stdout().flush().unwrap();
                }
            }
        }
        println!(
            "\n{}total tokens: {} = {} + {}{}",
            color::Fg(color::Yellow),
            usage.total_tokens,
            usage.prompt_tokens,
            usage.completion_tokens,
            color::Fg(color::Reset)
        );
    }
}
