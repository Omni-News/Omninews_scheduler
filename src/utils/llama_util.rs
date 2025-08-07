#![allow(dead_code)]

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

pub async fn query_llama_summarize(summarize_num: i32, phrase: &str) -> String {
    let prompt = format!(
        "다음 문장을 뉴스 기사 요약에 어울리는 객관적 서술체(‘~한다’, ‘~로 보인다’, ‘~라고 밝혔다’)로 요약해 주세요. \
    요약문은 {}자 이상 {}자 이하로 작성해 주세요.\n\n{}",
        summarize_num-10, summarize_num+10, phrase
    );

    let request_body = ChatRequest {
        model: "MLP-KTLim/llama-3-Korean-Bllossom-8B-gguf-Q4_K_M",
        max_tokens: 200,
        stream: false,
        messages: vec![Message {
            role: "user",
            content: &prompt,
        }],
    };

    let client = Client::new();
    let response = client
        .post("http://localhost:8080/v1/chat/completions")
        .json(&request_body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<ChatResponse>().await {
                    Ok(parsed) => {
                        let content = &parsed.choices[0].message.content;
                        content.to_string()
                    }
                    Err(e) => {
                        eprintln!("❌ JSON 파싱 실패: {}", e);
                        "본문 내용을 요약할 수 없습니다.".to_string()
                    }
                }
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eprintln!("❌ llama-server 응답 오류: {} - {}", status, body);
                "본문 내용을 요약할 수 없습니다.".to_string()
            }
        }
        Err(e) => {
            eprintln!("❌ 요청 실패: {}", e);
            "본문 내용을 요약할 수 없습니다.".to_string()
        }
    }
}
