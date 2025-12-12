# Omninews Scheduler

비동기 RSS 피드 수집 및 뉴스 스크래핑 스케줄러

## 개요

Omninews Scheduler는 다양한 RSS 채널과 네이버 뉴스로부터 뉴스 아이템을 자동으로 수집하고, AI 요약을 통해 데이터베이스에 저장하며, 구독자에게 푸시 알림을 전송하는 Rust 기반 스케줄러입니다.

- **웹사이트**: https://kang1027.com/omninews
- **App Store**: https://apps.apple.com/kr/app/omninews/id6746567181?l=en-GB

### 주요 기능

- **RSS 피드 수집**: 데이터베이스에 저장된 RSS 채널들의 아이템을 비동기로 가져와 저장
- **다양한 피드 포맷 지원**: RSS 2.0, Atom 등 주요 피드 포맷 모두 지원
- **네이버 뉴스 스크래핑**: Selenium을 활용한 네이버 뉴스 자동 수집
- **AI 기반 요약**: Google Gemini 2.0 Flash 모델을 활용한 뉴스 요약
- **스마트 푸시 알림**: 구독한 RSS 채널에 새로운 글이 올라왔을 때 실시간 알림 전송
- **비동기 처리**: Tokio 런타임을 활용한 효율적인 비동기 작업 처리

## 기술 스택

- **언어**: Rust (Edition 2021)
- **비동기 런타임**: Tokio
- **웹 프레임워크**: Rocket
- **데이터베이스**: MySQL (with SQLx)
- **RSS 파싱**: feed-rs, rss
- **웹 스크래핑**: Selenium (thirtyfour), scraper
- **AI 모델**: Google Gemini 2.0 Flash
- **인증**: JWT (jsonwebtoken)

## 설치 및 실행

### 1. 저장소 클론

```bash
git clone <repository-url>
cd Omninews_scheduler
```

### 2. 환경 변수 설정

`.env.example` 파일을 복사하여 `.env` 파일을 생성하고 필요한 값들을 설정합니다:

```bash
cp .env.example .env
```

#### 필수 환경 변수

**데이터베이스 설정**
- `DATABASE_URL`: MySQL 데이터베이스 연결 URL
  - 형식: `mysql://username:password@host:port/database_name`
- `MYSQL_ROOT_PASSWORD`: MySQL root 비밀번호 (Docker 사용 시)
- `MYSQL_DATABASE`: 데이터베이스 이름 (Docker 사용 시)
- `MYSQL_USER`: MySQL 사용자명 (Docker 사용 시)
- `MYSQL_PASSWORD`: MySQL 비밀번호 (Docker 사용 시)

**JWT 인증**
- `JWT_SECRET_KEY`: JWT 토큰 서명 키
  - **중요**: 백엔드 서버와 동일한 키를 사용해야 합니다

**Naver API**
- `NAVER_CLIENT_ID`: 네이버 개발자 센터에서 발급받은 Client ID
- `NAVER_CLIENT_SECRET`: 네이버 개발자 센터에서 발급받은 Client Secret

**Gemini API**
- `GEMINI_API_KEY`: Google AI Studio에서 발급받은 Gemini API 키
  - 뉴스 요약에 사용됩니다

**Apple App Store** (푸시 알림용)
- `APPLE_PRIVATE_KEY`: App Store Connect에서 생성한 Private Key
- `APPLE_KEY_ID`: Key ID
- `APPLE_ISSUER_ID`: Issuer ID
- `APPLE_BUNDLE_ID`: 앱 Bundle ID

**Instagram** (소셜 미디어 연동)
- `INSTAGRAM_ID`: Instagram 계정 ID
- `INSTAGRAM_PW`: Instagram 계정 비밀번호

**Selenium Scheduler** (웹 스크래핑용)
- `SCHEDULER_SELENIUM_URL_1` ~ `SCHEDULER_SELENIUM_URL_5`: Selenium 서버 URL들
  - 형식: `http://host:port`
  - 네이버 뉴스, RSS 피드, Instagram 등 지원하는 다양한 포맷의 콘텐츠 스크래핑에 사용됩니다

### 3. 빌드 및 실행

#### 로컬 환경에서 실행

SQLx는 컴파일 시 `DATABASE_URL`을 통해 데이터베이스와 연결하여 쿼리를 검증합니다. Docker 등 다른 배포 환경에서는 데이터베이스 접근이 제한될 수 있으므로, 사전 빌드를 진행합니다:

```bash
# 1. 로컬 데이터베이스 URL 설정 (동일한 스키마 사용)
export DATABASE_URL='mysql://username:password@127.0.0.1:3306/omninews'

# 2. SQLx 오프라인 모드 준비
cargo sqlx prepare -- --bin OmniNews

# 3. 실행
cargo run
```

#### Docker를 사용한 배포

```bash
# Docker Compose를 사용한 배포
docker compose -f docker-compose.dev.yml up -d
```

Docker 환경에서는 `.env` 파일의 설정이 자동으로 적용되며, SQLx 오프라인 모드를 통해 컴파일 시 데이터베이스 연결 없이도 빌드가 가능합니다.

## 프로젝트 구조

```
Omninews_scheduler/
├── src/                    # 소스 코드
├── logs/                   # 로그 파일
├── Cargo.toml             # Rust 프로젝트 설정
├── Rocket.toml            # Rocket 웹 프레임워크 설정
├── Dockerfile.dev         # 개발용 Docker 이미지
├── docker-compose.dev.yml # Docker Compose 설정
├── .env                   # 환경 변수 (git ignore)
├── .env.example           # 환경 변수 예시 파일
└── omninews_firebase_sdk.json  # Firebase SDK 설정
```

## 개발

### 로그

애플리케이션 로그는 `logs/` 디렉토리에 저장됩니다. 로그 레벨은 `RUST_LOG` 환경 변수로 조정할 수 있습니다:

```bash
RUST_LOG=debug cargo run
```

### 테스트

```bash
cargo test
```

### 코드 포맷팅

```bash
cargo fmt
```

### 린트

```bash
cargo clippy
```

## 주의사항

1. **JWT Secret Key**: 반드시 백엔드 서버와 동일한 `JWT_SECRET_KEY`를 사용해야 합니다.
2. **SQLx 오프라인 모드**: Docker 배포 전에 `cargo sqlx prepare` 명령을 실행하여 오프라인 모드 데이터를 생성해야 합니다.
3. **데이터베이스 스키마**: 데이터베이스가 올바른 스키마로 마이그레이션되어 있어야 합니다.
4. **API 키 관리**: `.env` 파일은 절대 Git에 커밋하지 마세요. `.gitignore`에 포함되어 있는지 확인하세요.
5. **Selenium 서버**: 웹 스크래핑 기능을 사용하려면 Selenium 서버가 실행 중이어야 합니다.

## 문제 해결

### SQLx 컴파일 오류

컴파일 시 데이터베이스 연결 오류가 발생하면:

```bash
# 로컬 데이터베이스에 연결 가능한 URL로 설정
export DATABASE_URL='mysql://username:password@127.0.0.1:3306/omninews'

# 오프라인 모드 데이터 생성
cargo sqlx prepare -- --bin OmniNews

# 이제 DATABASE_URL 없이도 빌드 가능
cargo build
```

### Docker 네트워크 오류

외부 네트워크 `omninews-prod-network`가 없다는 오류가 발생하면:

```bash
# 네트워크 생성
docker network create omninews-prod-network
```
