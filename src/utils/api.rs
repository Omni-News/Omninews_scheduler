#![allow(dead_code)]

use reqwest::Client;
use serde::{Deserialize, Serialize};

/*
*╰─ curl "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent" \
*  -H 'Content-Type: application/json' \
*  -H 'X-goog-api-key: AIzaSyBcTZbioNiLamuALOouItAm8JRsy9oEBvM' \
*  -X POST \
*  -d '{
*    "contents": [
*      {
*        "parts": [
*          {
*            "text": "ㅎㅇ"
*          }
*        ]
*      }
*    ]
*  }'
*
* */
#[derive(Serialize)]
struct ChatRequest {
    contents: Vec<Content>,
}

/*
*{
  "candidates": [
    {
      "content": {
        "parts": [
          {
            "text": "안녕하세요! 무엇을 도와드릴까요? 😊\n"
          }
        ],
        "role": "model"
      },
      "finishReason": "STOP",
      "avgLogprobs": -0.19659008085727692
    }
  ],
...
}
*/
#[derive(Serialize, Deserialize)]
struct ChatResponse {
    candidates: Vec<Candidate>,
}
#[derive(Serialize, Deserialize)]
struct Candidate {
    content: Content,
}
#[derive(Serialize, Deserialize)]
struct Content {
    parts: Vec<Parts>,
}
#[derive(Serialize, Deserialize)]
struct Parts {
    text: String,
}

pub async fn query_gemini_summarize(summarize_num: i32, phrase: &str) -> String {
    let prompt = format!(
        "아래 뉴스 기사를 요약에 어울리는 객관적 서술체(‘~한다’, ‘~로 보인다’, ‘~라고 밝혔다’)로 요약해 주세요. \
    요약문은 {}자 이상 {}자 이하로 작성해 주세요.\n\n{}",
        summarize_num-10, summarize_num+10, phrase
    );

    let request_body = ChatRequest {
        contents: vec![Content {
            parts: vec![Parts { text: prompt }],
        }],
    };

    let client = Client::new();
    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent")
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", "AIzaSyBcTZbioNiLamuALOouItAm8JRsy9oEBvM")
        .json(&request_body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<ChatResponse>().await {
                    Ok(parsed) => {
                        let content = &parsed.candidates[0].content.parts[0].text;
                        //info!("content: {}", content);
                        content.to_string()
                    }
                    Err(e) => {
                        eprintln!("❌ JSON 파싱 실패: {e}");
                        "본문 내용을 요약할 수 없습니다.".to_string()
                    }
                }
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eprintln!("❌ gemini-api 응답 오류: {status} - {body}");
                "본문 내용을 요약할 수 없습니다.".to_string()
            }
        }
        Err(e) => {
            eprintln!("❌ 요청 실패: {e}");
            "본문 내용을 요약할 수 없습니다.".to_string()
        }
    }
}
